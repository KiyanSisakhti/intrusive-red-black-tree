use crate::color::Color;
use core::ptr::NonNull;

/// Abstraction trait for intrusive Red-Black Tree nodes.
///
/// Implement this trait on custom node structures.
pub trait RBNode {
    /// The concrete node type managed by the tree (typically `Self`).
    type Node;

    /// The key type used for ordering nodes within the tree.
    type Key: Ord;

    /// Returns the parent node pointer of the target node.
    fn get_parent(node_ptr: NonNull<Self::Node>) -> NonNull<Self::Node>;

    /// Returns the current [`Color`] (Red or Black) of the target node.
    fn get_color(node_ptr: NonNull<Self::Node>) -> Color;

    /// Returns the child node pointer in the specified direction.
    ///
    /// # Direction Mapping
    /// * `0`: Left child
    /// * `1`: Right child
    fn get_child(node_ptr: NonNull<Self::Node>, dir: usize) -> NonNull<Self::Node>;

    /// Sets the child node pointer in the specified direction.
    ///
    /// # Direction Mapping
    /// * `0`: Left child
    /// * `1`: Right child
    fn set_child(node_ptr: NonNull<Self::Node>, dir: usize, child: NonNull<Self::Node>);

    /// Updates the parent pointer of the target node.
    fn set_parent(node_ptr: NonNull<Self::Node>, parent: NonNull<Self::Node>);

    /// Updates the [`Color`] of the target node.
    fn set_color(node_ptr: NonNull<Self::Node>, color: Color);

    /// Borrow a reference to the node's search/ordering key.
    fn get_key<'a>(node_ptr: NonNull<Self::Node>) -> &'a Self::Key;
}
