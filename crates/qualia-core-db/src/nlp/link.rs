//! Link only to known lexicon / ontology IDs.

use super::gazetteer::Hit;
use super::terms::known_iri;

pub fn filter_known(hits: Vec<Hit>) -> Vec<Hit> {
    hits.into_iter().filter(|h| known_iri(h.iri)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nlp::gazetteer::Gazetteer;

    #[test]
    fn drops_unknown_iri() {
        let g = Gazetteer::default();
        let hits = filter_known(g.find("North Spring"));
        assert!(hits.iter().all(|h| known_iri(h.iri)));
    }
}
