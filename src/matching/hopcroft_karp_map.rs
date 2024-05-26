use std::collections::HashMap;

use crate::profile::Output;
use crate::wl_backend::WlHead;

use super::{
    IntermediatePairing, IntermediatePairingWithMultipleModes, IntermediatePairingWithoutMode,
};

/// Collect `Head`s that matched to `Output`s in a bipartite graph, feed it into the
/// Hopcroft-Karp algorithm and return a maximum cardinality matching.
///
/// The hopcroft_karp crate expects that the vertices are the same type and implement Hash.
/// The crate chooses to panic if the graph is not bipartite.
/// Neither `Output` nor `Head` need to implement Hash and instead of relying on a correct
/// Hash implementation they will be mapped to a type that does: `isize`.
/// This is why this adapter exists.
///
/// It maps
/// - every `Output` to a positive integer > 0
/// - every `Head`   to a negative integer < 0
///
/// A graph that only contains edges between positive and negative integers is a bipartite graph.
pub struct HopcroftKarpMap;

impl HopcroftKarpMap {
    pub fn hkmap<Output, Head, E>(edges: impl Iterator<Item = E>) -> impl Iterator<Item = E>
    where
        Output: PartialEq + Clone,
        Head: PartialEq + Clone,
        E: Edge<Output, Head>,
    {
        let mut map = CombinedMap::default();

        // Map (Output, Head) edges to integer tuples
        let mapped_edges = edges.map(|e| map.insert(e)).collect();

        // Run Hopcroft-Karp on integer edges
        let matched_mapped_edges = hopcroft_karp::matching(&mapped_edges);

        // Map integers back to respective edge
        matched_mapped_edges
            .into_iter()
            .filter_map(move |me| map.pairs.remove(&me))
    }
}

struct CombinedMap<Output, Head, E>
where
    Output: PartialEq,
    Head: PartialEq,
    // E: AsEdgeRef<'p, Output, Head>,
    E: Edge<Output, Head>,
{
    // maps positive integers to Output
    mapping_output: LeftMap<Output>,
    // maps negative integers to Head
    mapping_head: RightMap<Head>,
    // Contains edges (positive, negative) integers
    pairs: HashMap<(isize, isize), E>,
}

impl<Output, Head, E> Default for CombinedMap<Output, Head, E>
where
    Output: PartialEq,
    Head: PartialEq,
    // E: AsEdgeRef<Output, Head>,
    E: Edge<Output, Head>,
{
    fn default() -> Self {
        Self {
            mapping_output: Default::default(),
            mapping_head: Default::default(),
            pairs: Default::default(),
        }
    }
}

impl<Output, Head, E> CombinedMap<Output, Head, E>
where
    Output: PartialEq + Clone,
    Head: PartialEq + Clone,
    E: Edge<Output, Head>,
{
    fn insert(&mut self, edge: E) -> (isize, isize) {
        let left = self.mapping_output.insert(edge.left().clone());
        let right = self.mapping_head.insert(edge.right().clone());
        self.pairs.insert((left, right), edge);
        (left, right)
    }
}

struct InnerMap<T: PartialEq> {
    list: Vec<T>,
}

/// Negative
struct LeftMap<T: PartialEq> {
    map: InnerMap<T>,
}

/// Positive
struct RightMap<T: PartialEq> {
    map: InnerMap<T>,
}

impl<T: PartialEq> Default for InnerMap<T> {
    fn default() -> Self {
        Self { list: vec![] }
    }
}

impl<T: PartialEq> Default for RightMap<T> {
    fn default() -> Self {
        Self {
            map: Default::default(),
        }
    }
}
impl<T: PartialEq> Default for LeftMap<T> {
    fn default() -> Self {
        Self {
            map: Default::default(),
        }
    }
}

impl<T: PartialEq> LeftMap<T> {
    fn insert(&mut self, value: T) -> isize {
        let index: usize = self.map.insert(value);
        let index: isize = -((index + 1) as isize);
        assert!(index < 0);
        index
    }
}
impl<T: PartialEq> RightMap<T> {
    fn insert(&mut self, value: T) -> isize {
        let index: usize = self.map.insert(value);
        let index: isize = (index + 1) as isize;
        assert!(index > 0);
        index
    }
}

impl<T: PartialEq> InnerMap<T> {
    fn insert(&mut self, value: T) -> usize {
        match self.list.iter().position(|t| *t == value) {
            Some(idx) => idx,
            None => {
                self.list.push(value);
                self.list.len() - 1
            }
        }
    }
}

/// Needed in the [`HopcroftKarpMap`].
///
/// This trait splits a type in two while keeping its inner information sealed and together.
pub trait Edge<L, R> {
    fn left(&self) -> &L;
    fn right(&self) -> &R;
}

impl Edge<Output, WlHead> for IntermediatePairing {
    fn left(&self) -> &Output {
        match self {
            IntermediatePairing::WithMultipleModes(ip) => ip.left(),
            IntermediatePairing::WithoutMode(ip) => ip.left(),
        }
    }
    fn right(&self) -> &WlHead {
        match self {
            IntermediatePairing::WithMultipleModes(ip) => ip.right(),
            IntermediatePairing::WithoutMode(ip) => ip.right(),
        }
    }
}

impl Edge<Output, WlHead> for IntermediatePairingWithMultipleModes {
    fn left(&self) -> &Output {
        &self.output
    }
    fn right(&self) -> &WlHead {
        &self.matched_head
    }
}
impl Edge<Output, WlHead> for IntermediatePairingWithoutMode {
    fn left(&self) -> &Output {
        &self.output
    }
    fn right(&self) -> &WlHead {
        &self.matched_head
    }
}
