use crate::model::event::{Event, EventLog, EventClassifier, Lifecycle, Trace};
use quick_xml::Reader;
use quick_xml::events::Event as XmlEvent;
use std::{fs::File, io::BufReader, path::Path};

#[derive(thiserror::Error, Debug)]
pub enum XesError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("XML parse error: {0}")]
    Xml(#[from] quick_xml::Error),

    #[error("XML attribute error: {0}")]
    XmlAttr(#[from] quick_xml::events::attributes::AttrError),

    #[error("not a valid XES file: missing <log> element")]
    NotXes,

    #[error("invalid XES structure: {0}")]
    Structure(String),

    #[error("event is missing its activity attribute")]
    MissingActivity,

    #[error("invalid timestamp: {0}")]
    Timestamp(#[from] chrono::ParseError),
}

pub fn read_xes(path: &Path, classifier: &EventClassifier) -> Result<EventLog, XesError> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    read_xes_from_reader(reader, classifier)
}

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

    #[test]
    fn reads_xes_from_file() {
        let path = Path::new("tests/test.xes");
        let classifier = EventClassifier::default();
        let log = read_xes(path, &classifier).unwrap();
        assert_eq!(log.traces.len(), 1000);
        assert_eq!(log.traces[0].events.len(), 6);
    }
}
