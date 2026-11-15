//! Canonical member identities after validated inheritance merging (MLS §5.6.1.4).

use rumoca_core::DefId;
use rumoca_ir_ast::{self as ast, AstIndexMap as IndexMap};
use std::collections::{HashMap, HashSet};

pub(crate) fn record_equivalence(
    equivalences: &mut Vec<(DefId, DefId)>,
    incoming: Option<DefId>,
    retained: Option<DefId>,
) {
    if let (Some(incoming), Some(retained)) = (incoming, retained)
        && incoming != retained
    {
        equivalences.push((incoming, retained));
    }
}

pub(crate) fn for_template(
    effective: &IndexMap<String, ast::Component>,
    inherited: &IndexMap<String, ast::Component>,
    equivalences: &[(DefId, DefId)],
) -> IndexMap<DefId, DefId> {
    let mut edges: HashMap<DefId, Vec<DefId>> = HashMap::new();
    let own_overrides = effective
        .iter()
        .filter_map(|(name, component)| Some((inherited.get(name)?.def_id?, component.def_id?)));
    for (a, b) in equivalences.iter().copied().chain(own_overrides) {
        if a != b {
            edges.entry(a).or_default().push(b);
            edges.entry(b).or_default().push(a);
        }
    }
    let mut result = IndexMap::default();
    for component in effective.values() {
        let Some(canonical) = component.def_id else {
            continue;
        };
        let mut pending = vec![canonical];
        let mut seen = HashSet::new();
        while let Some(declaration) = pending.pop() {
            if !seen.insert(declaration) {
                continue;
            }
            if declaration != canonical {
                result.insert(declaration, canonical);
            }
            pending.extend(edges.get(&declaration).into_iter().flatten().copied());
        }
    }
    result
}
