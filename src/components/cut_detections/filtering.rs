use super::{CutContext, CutStrategy};
use crate::components::filtering::FilteringConfig;
use crate::framework::DetectCut;
use crate::{IndexedCut, IndexedDfg, IndexedEventLog};
use std::collections::HashMap;

///
/// Filters edges relative to the maximum outgoing frequency of their source.
///
pub fn filter_dfg(graph: &IndexedDfg, config: FilteringConfig) -> IndexedDfg {
    let f = config.deviation_threshold();
    let mut maxima = HashMap::new();
    for (&(source, _), &count) in &graph.edges {
        maxima
            .entry(source)
            .and_modify(|max: &mut usize| *max = (*max).max(count))
            .or_insert(count);
    }
    for (&source, &count) in &graph.end_activities {
        maxima
            .entry(source)
            .and_modify(|max| *max = (*max).max(count))
            .or_insert(count);
    }
    let mut result = graph.clone();
    result
        .edges
        .retain(|(source, _), count| *count as f64 >= f * maxima[source] as f64);
    result
        .end_activities
        .retain(|source, count| *count as f64 >= f * maxima[source] as f64);
    let start_max = graph.start_activities.values().copied().max().unwrap_or(0);
    result
        .start_activities
        .retain(|_, count| *count as f64 >= f * start_max as f64);
    result
}

///
/// Try the unfiltered strategy chain first, then retry on a filtered DFG.
/// Both passes use the original log's minimum self-distance witnesses.
///
#[derive(Debug, Clone, Copy, Default)]
pub struct FilteringCutFinder<S> {
    pub strategy: S,
    pub config: FilteringConfig,
}
impl<S: CutStrategy> DetectCut for FilteringCutFinder<S> {
    fn detect_cut(&self, log: &IndexedEventLog) -> Option<IndexedCut> {
        let context = CutContext::new(log)?;
        if let Some(cut) = self.strategy.detect(&context) {
            return Some(cut);
        }
        let filtered = context.with_dfg(filter_dfg(context.dfg(), self.config));
        self.strategy.detect(&filtered)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::algorithms::im::ImCutFinder;
    use crate::test_support::indexed_log as log;
    use crate::{Activity, OperatorType};

    #[test]
    fn filtering_uses_local_maxima_boundary_edges_and_strict_comparison() {
        let input = log(&[&["a", "b"], &["c"]]);
        let a = input.interner().id(&Activity::from("a")).unwrap();
        let b = input.interner().id(&Activity::from("b")).unwrap();
        let c = input.interner().id(&Activity::from("c")).unwrap();
        let mut graph = IndexedDfg::from_log(&input);
        graph.edges = HashMap::from([((a, b), 10), ((a, c), 2), ((b, c), 1)]);
        graph.start_activities = HashMap::from([(a, 10), (b, 2), (c, 1)]);
        graph.end_activities = HashMap::from([(a, 1), (b, 10), (c, 1)]);
        let filtered = filter_dfg(&graph, FilteringConfig::new(0.2).unwrap());
        assert_eq!(filtered.edges, HashMap::from([((a, b), 10), ((a, c), 2)]));
        assert_eq!(filtered.start_activities, HashMap::from([(a, 10), (b, 2)]));
        assert_eq!(filtered.end_activities, HashMap::from([(b, 10), (c, 1)]));
        assert_eq!(filtered.activities, graph.activities);
        assert_eq!(graph.edges.len(), 3);
        assert_eq!(filter_dfg(&graph, FilteringConfig::new(0.0).unwrap()), graph);
    }
    #[test]
    fn unfiltered_cuts_take_precedence_even_at_threshold_one() {
        let input = log(&[&["a", "b"], &["a", "b", "c"]]);
        let strategy = ImCutFinder::default().0;
        let finder = FilteringCutFinder {
            strategy,
            config: FilteringConfig::new(1.0).unwrap(),
        };
        assert_eq!(
            finder.detect_cut(&input),
            ImCutFinder::default().detect_cut(&input)
        );
    }
    #[test]
    fn filtering_finds_a_sequence_hidden_by_rare_reverse_behavior() {
        let mut words = vec![&["a", "b"][..]; 10];
        words.extend([&["b", "a"][..], &["a", "b", "a"][..]]);
        let input = log(&words);
        assert!(ImCutFinder::default().detect_cut(&input).is_none());
        let finder = FilteringCutFinder {
            strategy: ImCutFinder::default().0,
            config: FilteringConfig::new(0.3).unwrap(),
        };
        let cut = finder.detect_cut(&input).unwrap();
        assert_eq!(cut.operator, OperatorType::Sequence);
        cut.validate(&input.activities()).unwrap();
        let disabled = FilteringCutFinder {
            strategy: ImCutFinder::default().0,
            config: FilteringConfig::new(0.0).unwrap(),
        };
        assert!(disabled.detect_cut(&input).is_none());
    }
}
