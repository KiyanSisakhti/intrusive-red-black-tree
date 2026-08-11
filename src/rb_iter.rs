use crate::{rb_node::RBNode, rb_tree::RBTree};
use core::ptr::NonNull;

/// An in-order iterator over the nodes of an [`RBTree`].
///
/// Yields raw pointers (`NonNull<T>`) to the nodes in strictly ascending key order,
/// starting from the minimum node up to the maximum node in the tree.
///
/// # Lifetimes
/// * `'a`: Lifetime bound by the immutable reference to the backing [`RBTree`].
pub struct RBTreeIter<'a, T>
where
    T: RBNode<Node = T>,
{
    /// Immutable reference to the backing tree structure.
    pub(crate) tree: &'a RBTree<T>,
    /// Pointer to the current node in the traversal sequence, or `None` if exhausted.
    pub(crate) curr: Option<NonNull<T::Node>>,
}

impl<'a, T: RBNode> Iterator for RBTreeIter<'a, T>
where
    T: RBNode<Node = T>,
{
    type Item = NonNull<T>;

    /// Advances the iterator and returns the pointer to the next node in ascending key order.
    ///
    /// Returns `None` once all nodes have been visited.
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let yield_node = self.curr?;
        self.curr = self.tree.next_node(yield_node);
        Some(yield_node)
    }
}
