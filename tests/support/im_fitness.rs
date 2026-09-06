use super::library::{EventLog, IM, LeafType, Miner, Node, OperatorType};
#[cfg(test)]
use std::collections::HashSet;

type Language = HashSet<Vec<String>>;
fn concat(a: &Language, b: &Language, max: usize) -> Language {
    a.iter()
        .flat_map(|x| {
            b.iter().filter_map(move |y| {
                if x.len() + y.len() > max {
                    None
                } else {
                    Some(x.iter().chain(y).cloned().collect())
                }
            })
        })
        .collect()
}
fn shuffles(a: &[String], b: &[String]) -> Language {
    if a.is_empty() {
        return HashSet::from([b.to_vec()]);
    }
    if b.is_empty() {
        return HashSet::from([a.to_vec()]);
    }
    let mut result = HashSet::new();
    for mut s in shuffles(&a[1..], b) {
        s.insert(0, a[0].clone());
        result.insert(s);
    }
    for mut s in shuffles(a, &b[1..]) {
        s.insert(0, b[0].clone());
        result.insert(s);
    }
    result
}
pub fn language(n: &Node, max: usize) -> Language {
    match n {
        Node::Leaf(l) => HashSet::from([match &l.activity_label {
            LeafType::Tau => vec![],
            LeafType::Activity(a) => vec![a.0.clone()],
        }]),
        Node::Operator(o) => {
            let children: Vec<_> = o.children.iter().map(|n| language(n, max)).collect();
            match o.operator_type {
                OperatorType::Xor => children.into_iter().flatten().collect(),
                OperatorType::Sequence => children
                    .iter()
                    .fold(HashSet::from([vec![]]), |a, b| concat(&a, b, max)),
                OperatorType::Concurrent => {
                    children.iter().fold(HashSet::from([vec![]]), |a, b| {
                        a.iter()
                            .flat_map(|x| {
                                b.iter()
                                    .filter(|y| x.len() + y.len() <= max)
                                    .flat_map(|y| shuffles(x, y))
                            })
                            .collect()
                    })
                }
                OperatorType::Loop => {
                    let redo: Language = children[1..]
                        .iter()
                        .flat_map(|l| l.iter().cloned())
                        .collect();
                    let extension = concat(&redo, &children[0], max);
                    let mut result = children[0].clone();
                    loop {
                        let next = concat(&result, &extension, max);
                        let before = result.len();
                        result.extend(next);
                        if result.len() == before {
                            break;
                        }
                    }
                    result
                }
                _ => panic!("standard IM must use only four operators"),
            }
        }
    }
}
pub fn check_fitness(l: &EventLog) {
    let tree = IM::default().mine(l).unwrap();
    let max = l.traces.iter().map(|t| t.events.len()).max().unwrap_or(0);
    let accepted = language(tree.root(), max);
    for t in &l.traces {
        let sequence: Vec<_> = t.events.iter().map(|e| e.activity.0.clone()).collect();
        assert!(
            accepted.contains(&sequence),
            "missing {sequence:?} in {tree:?}"
        );
    }
}
