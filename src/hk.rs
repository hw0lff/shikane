use std::collections::HashMap;

use crate::backend::output_head::OutputHead;
use crate::backend::ShikaneBackend;
use crate::profile::Output;

use hopcroft_karp as hk;

/// Collect [`OutputHead`]s that matched to [`Output`]s in a bipartite graph, feed it into the
/// Hopcroft-Karp algorithm and return a maximum cardinality matching
///
/// The hopcroft_karp crate expects that the vertices are the same type and implement Hash.
/// The crate chooses to panic if the graph is not bipartite.
/// Neither [`Output`] nor [`OutputHead`] implement Hash and instead of relying on a correct
/// Hash implementation they will be mapped to a type that does: [`isize`].
/// This is why this adapter exists.
/// It maps
/// - every [`Output`]     to a positive integer > 0
/// - every [`OutputHead`] to a negative integer < 0
/// A graph that only contains edges between positive and negative integers is a bipartite
/// graph.
pub struct HKMap<'a, 'b> {
    /// Maps positive integers to [`Output`]
    mapping_output: HashMap<isize, &'a Output>,
    /// Maps negative integers to [`OutputHead`]
    mapping_head: HashMap<isize, &'b OutputHead>,
    /// Contains edges (positive, negative) integers
    hk_edges: Vec<(isize, isize)>,
}

impl<'a, 'b> HKMap<'a, 'b> {
    pub fn new(outputs: &'a [Output], backend: &'b ShikaneBackend) -> Self {
        // maps positive integers to Output
        let mapping_output: HashMap<isize, &Output> =
            (1..=isize::MAX).zip(outputs.iter()).collect();
        // maps negative integers to OutputHead
        let mapping_head: HashMap<isize, &OutputHead> = (isize::MIN..=-1)
            .rev()
            .zip(backend.output_heads.iter().map(|(_, o_head)| o_head))
            .collect();

        Self {
            mapping_output,
            mapping_head,
            hk_edges: vec![],
        }
    }

    pub fn create_hk_matchings(
        &mut self,
        outputs: &'a [Output],
        backend: &'b ShikaneBackend,
    ) -> Vec<(&'a Output, &'b OutputHead)> {
        // Create the edges
        for next_o in outputs.iter() {
            for next_h in backend.match_heads(next_o).iter() {
                // p .. positive
                if let Some((&p, _)) = self.mapping_output.iter().find(|(_, o)| next_o == **o) {
                    // n .. negative
                    if let Some((&n, _)) = self.mapping_head.iter().find(|(_, h)| next_h == *h) {
                        self.hk_edges.push((p, n));
                    };
                }
            }
        }

        // Run Hopcroft-Karp
        let hk_matchings = hk::matching(&self.hk_edges);

        // Map integers back to correct types
        let back_mapped_matchings: Vec<(&Output, &OutputHead)> = hk_matchings
            .iter()
            .filter_map(|(hk_p, hk_n)| {
                if let Some(o) = self.mapping_output.get(hk_p) {
                    if let Some(h) = self.mapping_head.get(hk_n) {
                        return Some((*o, *h));
                    }
                }
                None
            })
            .collect();

        back_mapped_matchings
    }
}
