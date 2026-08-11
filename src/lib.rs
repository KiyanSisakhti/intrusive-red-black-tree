#![no_std]

//! # Intrusive Red-Black Tree (`no_std`)
//!
//! A high-performance, zero-allocation, intrusive Red-Black Tree implementation
//! engineered specifically for `#![no_std]` systems, operating system kernels, bare-metal
//! runtimes, and low-level memory allocators.
//!
//! Unlike standard container structures (such as `std::collections::BTreeMap`), this crate uses an **intrusive layout**.
//! Nodes are allocated and owned by the caller (e.g., embedded directly inside page descriptors, Virtual Memory Area
//! structs, or slab headers), and linked into the tree via pointer references (`NonNull<T>`).
//!
//! ---
//!
//! ## Key Architectural Features
//!
//! * **Intrusive Trait Abstraction ([`RBNode`]):** decouples memory layout from tree logic. Implement [`RBNode`] on any custom struct to store metadata directly within your allocation.
//! * **Zero Heap Allocation:** operations (`insert`, `delete`, `search`, `iter`) perform zero internal allocations or dynamic memory management.
//! * **Sentinel Node Pattern (`nil`):** uses a dedicated sentinel node representing leaves and tree boundaries. This eliminates null-pointer branch checks in critical balance/rotation loops.
//! * **O(1) Cached Minimum:** tracks the leftmost node dynamically during insertions and deletions for instant O(1) minimum key access and O(1) start for in-order iteration.
//! * **Flexible Querying (`Borrow<Q>`):** enables zero-allocation searches on complex keys (e.g., querying a node with a `String` key using a borrowed `&str`).
//! * **Bitwise Optimized Rotations:** standardizes directional logic (`0` for left, `1` for right) using bitwise operations (`dir ^ 1`), reducing code duplicate paths for symmetric rebalancing logic.
//!
//! ---
//!
//! ## Red-Black Tree Invariants
//!
//! Every tree managed by [`RBTree`] strictly maintains the 5 fundamental Red-Black invariants across all operations:
//!
//! 1. **Node Color:** Every node is either [`Color::RED`] or [`Color::BLACK`].
//! 2. **Root Property:** The root node is always [`Color::BLACK`].
//! 3. **Leaf Property:** Every leaf (represented by the sentinel `nil` node) is [`Color::BLACK`].
//! 4. **Red Violation Absence:** If a node is [`Color::RED`], both of its children must be [`Color::BLACK`] (no two adjacent red nodes).
//! 5. **Black Height Uniformity:** Every path from a given node to any of its descendant `nil` leaves contains the exact same number of black nodes.
//!
//! ---
//!
//! ## Algorithmic Complexity
//!
//! | Operation | Time Complexity | Space Complexity |
//! | :--- | :--- | :--- |
//! | Search / Query (`search`, `exist`) | $\mathcal{O}(\log N)$ | $\mathcal{O}(1)$ |
//! | Insertion (`insert`) | $\mathcal{O}(\log N)$ | $\mathcal{O}(1)$ |
//! | Deletion (`delete`) | $\mathcal{O}(\log N)$ | $\mathcal{O}(1)$ |
//! | Pop Minimum (`delete_min`) | $\mathcal{O}(\log N)$ worst, $\mathcal{O}(1)$ expected | $\mathcal{O}(1)$ |
//! | Access Minimum Node (`min_node`) | $\mathcal{O}(1)$ | $\mathcal{O}(1)$ |
//! | In-order Iteration (`iter`, `next_node`) | $\mathcal{O}(1)$ amortized per step | $\mathcal{O}(1)$ |
//!
//! ---
//!
//! ## Safety & Ownership Obligations
//!
//! Because this crate works directly with raw `NonNull<T>` pointers:
//!
//! 1. **Pinning & Address Stability:** Nodes inserted into an [`RBTree`] must remain at stable memory locations until unlinked or deleted. Moving a node in memory invalidates tree pointers.
//! 2. **Lifetime Management:** Deallocating a node while it is still linked inside an active [`RBTree`] causes Undefined Behavior (UB). Always call [`RBTree::delete`] or [`RBTree::delete_min`] before dropping a node.
//! 3. **Sentinel Validity:** The sentinel node (`nil`) passed to [`RBTree::new`] must remain valid and initialized for the entire lifespan of the tree.
//!
//! ---
//!
//! ## Complete & Testable Example
//!
//! The following testable example demonstrates defining a custom memory node, implementing [`RBNode`],
//! setting up the tree with a sentinel node, performing insertions, searching, iterating in-order, and clean deletion.
//!
//! ```rust
//! use core::ptr::NonNull;
//! use intrusive_red_black_tree::{Color, RBNode, RBTree};
//!
//! // 1. Define your domain struct containing tree linkage pointers.
//! #[derive(Debug)]
//! pub struct VmaNode {
//!     pub start_addr: usize,
//!     pub end_addr: usize,
//!     pub color: Color,
//!     pub parent: NonNull<Self>,
//!     pub children: [NonNull<Self>; 2],
//! }

//! impl VmaNode {
//!     /// Helper to create a heap/leak-allocated node for testing.
//!     pub fn new(start_addr: usize, end_addr: usize, sentinel: NonNull<Self>) -> NonNull<Self> {
//!         let boxed = Box::new(Self {
//!             start_addr,
//!             end_addr,
//!             color: Color::RED,
//!             parent: sentinel,
//!             children: [sentinel, sentinel],
//!         });
//!         NonNull::from(Box::leak(boxed))
//!     }
//!
//!     /// Helper to construct a valid sentinel node.
//!     pub fn create_sentinel() -> NonNull<Self> {
//!         let sentinel_raw = Box::leak(Box::new(Self {
//!             start_addr: 0,
//!             end_addr: 0,
//!             color: Color::BLACK,
//!             parent: NonNull::dangling(),
//!             children: [NonNull::dangling(), NonNull::dangling()],
//!         }));
//!         let mut ptr = NonNull::from(sentinel_raw);
//!         unsafe {
//!             ptr.as_mut().parent = ptr;
//!             ptr.as_mut().children = [ptr, ptr];
//!         }
//!         ptr
//!     }
//!
//!     /// Safely clean up leaked heap nodes in test/drop routines.
//!     pub unsafe fn free(ptr: NonNull<Self>) {
//!         unsafe { drop(Box::from_raw(ptr.as_ptr())) };
//!     }
//! }
//!
//! // 2. Implement the `RBNode` trait for your structure.
//! impl RBNode for VmaNode {
//!     type Node = VmaNode;
//!     type Key = usize;
//!
//!     fn set_color(mut node_ptr: NonNull<Self>, color: Color) {
//!         unsafe { node_ptr.as_mut() }.color = color;
//!     }
//!
//!     fn get_color(node_ptr: NonNull<Self>) -> Color {
//!         unsafe { node_ptr.as_ref() }.color
//!     }
//!
//!     fn set_parent(mut node_ptr: NonNull<Self>, parent: NonNull<Self>) {
//!         unsafe { node_ptr.as_mut() }.parent = parent;
//!     }
//!
//!     fn get_parent(node_ptr: NonNull<Self>) -> NonNull<Self> {
//!         unsafe { node_ptr.as_ref() }.parent
//!     }
//!
//!     fn set_child(mut node_ptr: NonNull<Self>, dir: usize, child: NonNull<Self>) {
//!         unsafe { node_ptr.as_mut() }.children[dir] = child;
//!     }
//!
//!     fn get_child(node_ptr: NonNull<Self>, dir: usize) -> NonNull<Self> {
//!         unsafe { node_ptr.as_ref() }.children[dir]
//!     }
//!
//!     fn get_key<'a>(node_ptr: NonNull<Self>) -> &'a Self::Key {
//!         unsafe { &node_ptr.as_ref().start_addr }
//!     }
//! }
//!
//! // 3. Drive tree operations.
//! fn main() {
//!     let sentinel = VmaNode::create_sentinel();
//!     let mut tree = RBTree::new(sentinel);
//!
//!     // Allocate nodes
//!     let node1 = VmaNode::new(0x1000, 0x2000, sentinel);
//!     let node2 = VmaNode::new(0x0500, 0x0FFF, sentinel);
//!     let node3 = VmaNode::new(0x3000, 0x5000, sentinel);
//!
//!     // Insert nodes
//!     assert!(tree.insert(node1));
//!     assert!(tree.insert(node2));
//!     assert!(tree.insert(node3));
//!
//!     // Duplicate insertion should fail
//!     assert!(!tree.insert(node1));
//!
//!     // Query minimum
//!     assert_eq!(tree.min_node(), node2);
//!
//!     // Zero-allocation search
//!     let found = tree.search(&0x1000);
//!     assert_eq!(found, Some(node1));
//!     assert!(tree.exist(&0x3000));
//!     assert!(!tree.exist(&0x9000));
//!
//!     // In-order iteration (returns nodes sorted by key: 0x0500 -> 0x1000 -> 0x3000)
//!     let keys: Vec<usize> = tree.iter().map(|n| *VmaNode::get_key(n)).collect();
//!     assert_eq!(keys, vec![0x0500, 0x1000, 0x3000]);
//!
//!     // Deletion
//!     let deleted = tree.delete(&0x1000);
//!     assert_eq!(deleted, Some(node1));
//!     assert!(!tree.exist(&0x1000));

//!     // Free allocations
//!     unsafe {
//!         VmaNode::free(node1);
//!         VmaNode::free(node2);
//!         VmaNode::free(node3);
//!         VmaNode::free(sentinel);
//!     }
//! }
//! ```

mod color;
mod rb_iter;
mod rb_node;
mod rb_tree;

pub use color::Color;
pub use rb_node::RBNode;
pub use rb_tree::RBTree;
