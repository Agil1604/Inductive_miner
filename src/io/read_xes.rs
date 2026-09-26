use crate::model::event::{Event, EventClassifier, EventLog, Lifecycle, Trace};
use quick_xml::Reader;
use quick_xml::events::Event as XmlEvent;
use std::{fs::File, io::BufReader, path::Path};

/// Errors returned by [`read_xes`] and [`read_xes_from_reader`].
///
/// Distinguishes file access, XML parsing, unsupported XES structure, and
/// invalid or missing event data. This reader checks the structure it needs
/// to construct an event log; it does not perform full XES schema validation.
#[derive(thiserror::Error, Debug)]
pub enum XesError {
    /// Opening the input file failed.
    ///
    /// I/O failures encountered while parsing a reader are reported through
    /// [`Self::Xml`] by the XML parser.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// The XML parser could not read or decode the document or attribute values.
    #[error("XML parse error: {0}")]
    Xml(#[from] quick_xml::Error),

    /// An XML attribute is malformed, for example because its value is unquoted
    /// or its name occurs more than once on the same element.
    #[error("XML attribute error: {0}")]
    XmlAttr(#[from] quick_xml::events::attributes::AttrError),

    /// Parsing reached the end of the document without finding a `<log>` element.
    #[error("not a valid XES file: missing <log> element")]
    NotXes,

    /// The document violates the reader's expected element structure.
    ///
    /// The contained message describes the problem, such as a trace outside
    /// the log, an event outside a trace, or an unclosed element at end of input.
    #[error("invalid XES structure: {0}")]
    Structure(String),

    /// An event has no attribute matching [`EventClassifier::activity_key`],
    /// even after applying global event defaults.
    #[error("event is missing its activity attribute")]
    MissingActivity,

    /// A present timestamp selected by [`EventClassifier::timestamp_key`] could
    /// not be parsed as RFC 3339. Missing timestamps do not produce this error.
    #[error("invalid timestamp: {0}")]
    Timestamp(#[from] chrono::ParseError),
}

/// Reads an XES file into an event log using buffered file access.
///
/// `path` identifies the file to open. `classifier` selects the event attributes
/// used for activity names, timestamps, and lifecycle transitions; its default
/// uses `concept:name`, `time:timestamp`, and `lifecycle:transition`.
/// See [`read_xes_from_reader`] for parsing and metadata handling.
///
/// # Errors
/// Returns [`XesError::Io`] if the file cannot be opened, or a parsing error
/// from [`read_xes_from_reader`] if its contents cannot be read as an event log.
///
/// # Examples
/// ```no_run
/// use std::path::Path;
/// use inductive_miner::{EventClassifier, io::read_xes};
///
/// let log = read_xes(Path::new("log.xes"), &EventClassifier::default())?;
/// println!("Read {} traces", log.traces.len());
/// # Ok::<(), inductive_miner::io::read_xes::XesError>(())
/// ```
pub fn read_xes(path: &Path, classifier: &EventClassifier) -> Result<EventLog, XesError> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    read_xes_from_reader(reader, classifier)
}

/// Parses XES file from a buffered reader into an event log.
///
/// Accepts any [`std::io::BufRead`]. Traces and events retain their document order,
/// duplicates, and empty traces. Events are neither sorted by timestamp nor filtered
/// by lifecycle; use [`EventLog::atomic`] to project lifecycle events to atomic executions.
///
/// `classifier` selects activity, timestamp, and lifecycle attribute keys.
/// Direct attributes in `<global scope="event">` provide defaults for events
/// read afterward; an event's own direct attributes override those defaults.
/// Activity names are required. Absent timestamps become `None`, while absent
/// lifecycle values become [`Lifecycle::Unknown`]. Setting an optional classifier
/// key to `None` disables reading that field. Present timestamps are parsed as
/// RFC 3339 and converted to UTC.
///
/// A trace's direct `concept:name` attribute becomes its case ID, defaulting to
/// an empty string when absent. Other metadata and nested attribute values are
/// ignored. The reader checks the structure needed to construct the log, rather
/// than validating the complete XES schema.
///
/// # Errors
/// Returns [`XesError::Xml`] or [`XesError::XmlAttr`] for XML parsing failures
/// (including input read failures), [`XesError::NotXes`] when no `<log>` is found,
/// or [`XesError::Structure`] for invalid element placement or unclosed elements.
/// Returns [`XesError::MissingActivity`] when an event lacks the selected activity
/// attribute after defaults, or [`XesError::Timestamp`] for an invalid selected
/// timestamp.
///
/// # Examples
/// ```
/// use inductive_miner::{EventClassifier, io::read_xes_from_reader};
///
/// let xml = br#"<log><trace>
///     <string key="concept:name" value="case-1"/>
///     <event><string key="concept:name" value="A"/></event>
/// </trace></log>"#;
/// let log = read_xes_from_reader(xml.as_slice(), &EventClassifier::default())?;
/// assert_eq!(log.traces[0].case_id, "case-1");
/// assert_eq!(log.traces[0].events[0].activity.0, "A");
/// # Ok::<(), inductive_miner::io::read_xes::XesError>(())
/// ```
pub fn read_xes_from_reader<R: std::io::BufRead>(
    reader: R,
    classifier: &EventClassifier,
) -> Result<EventLog, XesError> {
    let mut xml = Reader::from_reader(reader);
    xml.config_mut().trim_text(true);

    let mut buf = Vec::new();
    let mut log = EventLog::default();
    let mut stack: Vec<String> = Vec::new();
    let mut saw_log = false;
    let mut trace: Option<Trace> = None;
    let mut attributes = std::collections::HashMap::<String, String>::new();
    let mut event_defaults = std::collections::HashMap::<String, String>::new();
    let mut global_events = false;

    loop {
        let token = xml.read_event_into(&mut buf)?;
        match token {
            XmlEvent::Start(ref e) | XmlEvent::Empty(ref e) => {
                let name = e.local_name().as_ref().to_owned();
                let parent = stack.last().map(String::as_str);
                let mut attrs = std::collections::HashMap::new();
                for attr in e.attributes() {
                    let attr = attr?;
                    let key = attr.key.as_ref().to_owned();
                    let value = attr
                        .normalized_value(quick_xml::XmlVersion::Implicit1_0)?
                        .into_owned();
                    attrs.insert(key, value);
                }
                match name.as_str() {
                    "log" => {
                        if saw_log || parent.is_some() {
                            return Err(XesError::Structure("unexpected log element".into()));
                        }
                        saw_log = true;
                    }
                    "trace" => {
                        if parent != Some("log") {
                            return Err(XesError::Structure("trace outside log".into()));
                        }
                        trace = Some(Trace::default());
                    }
                    "event" => {
                        if parent != Some("trace") {
                            return Err(XesError::Structure("event outside trace".into()));
                        }
                        attributes = event_defaults.clone();
                    }
                    "global" if parent == Some("log") => {
                        global_events = attrs.get("scope").is_some_and(|v| v == "event");
                    }
                    _ => {
                        if let (Some(key), Some(value)) = (attrs.get("key"), attrs.get("value")) {
                            match parent {
                                Some("event") => {
                                    attributes.insert(key.clone(), value.clone());
                                }
                                Some("global") if global_events => {
                                    event_defaults.insert(key.clone(), value.clone());
                                }
                                Some("trace") if key == "concept:name" => {
                                    trace.as_mut().unwrap().case_id = value.clone();
                                }
                                _ => {}
                            }
                        }
                    }
                }
                if matches!(token, XmlEvent::Empty(_)) {
                    finish_element(&name, &mut trace, &mut log, &attributes, classifier)?;
                    if name == "global" {
                        global_events = false;
                    }
                } else {
                    stack.push(name);
                }
            }
            XmlEvent::End(e) => {
                let name = e.local_name().as_ref().to_owned();
                if stack.pop().as_deref() != Some(name.as_str()) {
                    return Err(XesError::Structure("unmatched closing element".into()));
                }
                finish_element(&name, &mut trace, &mut log, &attributes, classifier)?;
                if name == "global" {
                    global_events = false;
                }
            }
            XmlEvent::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    if !saw_log {
        return Err(XesError::NotXes);
    }
    if !stack.is_empty() {
        return Err(XesError::Structure("unexpected end of file".into()));
    }
    Ok(log)
}

fn finish_element(
    name: &str,
    trace: &mut Option<Trace>,
    log: &mut EventLog,
    attributes: &std::collections::HashMap<String, String>,
    classifier: &EventClassifier,
) -> Result<(), XesError> {
    match name {
        "event" => {
            let activity = attributes
                .get(&classifier.activity_key)
                .ok_or(XesError::MissingActivity)?;
            let lifecycle = classifier
                .lifecycle_key
                .as_ref()
                .and_then(|key| attributes.get(key))
                .map(|value| Lifecycle::from_xes(value))
                .unwrap_or(Lifecycle::Unknown);
            let timestamp = classifier
                .timestamp_key
                .as_ref()
                .and_then(|key| attributes.get(key))
                .map(|value| {
                    chrono::DateTime::parse_from_rfc3339(value)
                        .map(|t| t.with_timezone(&chrono::Utc))
                })
                .transpose()?;
            trace
                .as_mut()
                .ok_or_else(|| XesError::Structure("event outside trace".into()))?
                .events
                .push(Event {
                    activity: crate::Activity(activity.clone()),
                    lifecycle,
                    timestamp,
                });
        }
        "trace" => log.traces.push(
            trace
                .take()
                .ok_or_else(|| XesError::Structure("unexpected trace end".into()))?,
        ),
        _ => {}
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_order_duplicates_empty_traces_and_lifecycle() {
        let input = r#"<log><trace><string key="concept:name" value="case &amp; 1"/>
          <event><string key="concept:name" value="a"/><string key="lifecycle:transition" value="start"/></event>
          <event><string key="concept:name" value="a"/><string key="lifecycle:transition" value="complete"/>
          <date key="time:timestamp" value="2026-10-01T12:00:00+03:00"/></event></trace>
          <trace><event><string key="concept:name" value="a"/></event></trace><trace/></log>"#;
        let log = read_xes_from_reader(input.as_bytes(), &EventClassifier::default()).unwrap();
        assert_eq!(log.traces.len(), 3);
        assert_eq!(log.traces[0].case_id, "case & 1");
        assert_eq!(log.traces[0].events.len(), 2);
        assert_eq!(
            log.traces[0].events[1].timestamp.unwrap().to_rfc3339(),
            "2026-10-01T09:00:00+00:00"
        );
        let atomic = log.atomic();
        assert_eq!(atomic.traces[0].events.len(), 1);
        assert_eq!(atomic.traces[1].events.len(), 1);
        assert!(atomic.traces[2].events.is_empty());
    }

    #[test]
    fn rejects_missing_log_activity_and_truncated_xml() {
        let classifier = EventClassifier::default();
        assert!(matches!(
            read_xes_from_reader(b"<other/>".as_slice(), &classifier),
            Err(XesError::NotXes)
        ));
        assert!(matches!(
            read_xes_from_reader(
                b"<log><trace><event/></trace></log>".as_slice(),
                &classifier
            ),
            Err(XesError::MissingActivity)
        ));
        assert!(read_xes_from_reader(b"<log><trace>".as_slice(), &classifier).is_err());
    }

    #[test]
    fn supports_custom_keys_and_global_defaults() {
        let input = r#"<log><global scope="event"><string key="task" value="default"/></global>
        <trace><event/><event><string key="task" value="override"/></event></trace></log>"#;
        let classifier = EventClassifier {
            activity_key: "task".into(),
            timestamp_key: None,
            lifecycle_key: None,
        };
        let log = read_xes_from_reader(input.as_bytes(), &classifier).unwrap();
        assert_eq!(log.traces[0].events[0].activity.0, "default");
        assert_eq!(log.traces[0].events[1].activity.0, "override");
    }

    #[test]
    fn supports_empty_log() {
        let input = r#"<log></log>"#;
        let classifier = EventClassifier::default();
        let log = read_xes_from_reader(input.as_bytes(), &classifier).unwrap();
        assert!(log.traces.is_empty());
    }

    #[test]
    fn supports_empty_trace() {
        let input = r#"<log><trace></trace></log>"#;
        let classifier = EventClassifier::default();
        let log = read_xes_from_reader(input.as_bytes(), &classifier).unwrap();
        assert_eq!(log.traces.len(), 1);
        assert!(log.traces[0].events.is_empty());
    }

    fn read_fixture(name: &str) -> EventLog {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("test_data/xes_examples")
            .join(name);
        read_xes(&path, &EventClassifier::default()).unwrap()
    }

    fn activity_names(trace: &Trace) -> Vec<&str> {
        trace.events.iter().map(|e| e.activity.0.as_str()).collect()
    }

    #[test]
    fn reads_five_traces_with_repetitions() {
        let log = read_fixture("five_traces_with_repetitions.xes");
        let expected = [
            vec!["A", "B", "C"],
            vec!["A", "C", "B"],
            vec!["A", "B", "B", "C"],
            vec!["A", "B", "C", "B", "C"],
            vec!["A", "B", "C"],
        ];
        assert_eq!(log.traces.len(), expected.len());
        for (trace, expected) in log.traces.iter().zip(expected) {
            assert_eq!(activity_names(trace), expected);
            assert!(trace.case_id.is_empty());
            assert!(
                trace
                    .events
                    .iter()
                    .all(|e| e.lifecycle == Lifecycle::Unknown && e.timestamp.is_none())
            );
        }
    }

    #[test]
    fn reads_nested_attributes_without_treating_metadata_as_events() {
        let log = read_fixture("nested_attributes.xes");
        assert_eq!(log.traces.len(), 2);
        assert_eq!(log.traces[0].case_id, "case-1");
        assert_eq!(log.traces[1].case_id, "case-2");
        assert_eq!(activity_names(&log.traces[0]), ["A", "B"]);
        assert_eq!(activity_names(&log.traces[1]), ["C"]);
        assert!(
            log.traces
                .iter()
                .flat_map(|t| &t.events)
                .all(|e| e.timestamp.is_none() && e.lifecycle == Lifecycle::Unknown)
        );
    }

    #[test]
    fn reads_three_cases_with_lifecycle() {
        let log = read_fixture("three_cases_with_lifecycle.xes");
        assert_eq!(log.traces.len(), 3);
        for (i, (activity, start, end)) in [
            ("A", "2026-10-02T09:00:00Z", "2026-10-02T09:01:00Z"),
            ("B", "2026-10-02T10:00:00Z", "2026-10-02T10:02:00Z"),
            ("C", "2026-10-02T11:00:00Z", "2026-10-02T11:03:00Z"),
        ]
        .into_iter()
        .enumerate()
        {
            let trace = &log.traces[i];
            assert_eq!(trace.case_id, format!("case-{}", i + 1));
            assert_eq!(activity_names(trace), [activity, activity]);
            assert_eq!(trace.events[0].lifecycle, Lifecycle::Start);
            assert_eq!(trace.events[1].lifecycle, Lifecycle::Complete);
            for (event, timestamp) in trace.events.iter().zip([start, end]) {
                assert_eq!(
                    event.timestamp,
                    Some(
                        chrono::DateTime::parse_from_rfc3339(timestamp)
                            .unwrap()
                            .with_timezone(&chrono::Utc)
                    )
                );
            }
        }
    }

    #[test]
    fn reads_pdc2025_1000_traces() {
        let log = read_fixture("pdc2025_1000_traces.xes");
        assert_eq!(log.traces.len(), 1000);
        assert_eq!(log.traces[0].events.len(), 6);
        assert_eq!(
            activity_names(&log.traces[0]),
            ["t252", "t396", "t580", "t936", "t482", "t969"]
        );
        assert_eq!(
            log.traces.iter().map(|t| t.events.len()).sum::<usize>(),
            16326
        );
        assert_eq!(log.traces[0].case_id, "trace 1");
        assert_eq!(log.traces[999].case_id, "trace 1000");
        assert!(
            log.traces
                .iter()
                .flat_map(|t| &t.events)
                .all(|e| !e.activity.0.is_empty()
                    && e.lifecycle == Lifecycle::Unknown
                    && e.timestamp.is_none())
        );
    }
}
