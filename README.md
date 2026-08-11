<!-- # intrusive-red-black-tree

[![Crates.io](https://img.shields.io/crates/v/intrusive-red-black-tree.svg)](https://crates.io/crates/intrusive-red-black-tree)
[![Documentation](https://docs.rs/intrusive-red-black-tree/badge.svg)](https://docs.rs/intrusive-red-black-tree)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![no_std](https://img.shields.io/badge/std-no-success.svg)](https://github.com/) -->

<div align="center">
  <h1>Intrusive Red-Black Tree</h1>
</br>
</br>

[![crate][crate-badge]][crate-link]
![Test Status][git-ci]
![Lines of Code][total-lines]

![Repo Size][repo-size]
[![MIT licensed][license-image]][license-link]
[![Docs][docs-image]][docs-link]
</div>

</br>
</br>
A deterministic, zero-allocation, intrusive Red-Black Tree implementation engineered specifically for #![no_std] environments, OS kernel memory managers (e.g., Virtual Memory Areas, Page Allocators), bare-metal runtimes, and low-latency embedded systems.

Unlike traditional containers like `std::collections::BTreeMap`, which own their elements and perform dynamic heap allocations, **`intrusive-red-black-tree`** embeds the tree linkage metadata directly inside the user structure. Memory allocation and lifetime management are completely controlled by the caller.

---

## Key Features

* **`#![no_std]` Native:** Zero dependencies on `std` or `alloc`. Fully compatible with bare-metal microcontrollers and hypervisor/kernel codebases.
* **Intrusive Architecture:** Nodes are stored directly inside your custom structures. No node wrappers, indirect pointer chasing, or hidden heap allocations.
* **Zero Overhead Iteration (`RBTreeIter`):** In-order traversal starting from the cached minimum node without dynamic memory or vector allocation.
* **O(1) Cached Minimum:** Maintains a dynamic pointer to the leftmost element, providing instant $\mathcal{O}(1)$ minimum key access.
* **Sentinel Node Design (`nil`):** Utilizes a static or per-tree sentinel node representing leaves and tree boundaries, eliminating null-pointer branch checks during rotations and rebalancing.
* **Flexible Key Borrowing (`Borrow<Q>`):** Search for items without instantiating full keys (e.g., query a node storing a complex key using a simple borrowed slice or integer reference).
* **Miri Verified:** Fully verified under `cargo miri` for strict adherence to Rust pointer aliasing rules, Stacked/Tree Borrows, and memory safety invariants.

---


## Crate Architecture & Module Layout

The crate consists of four core modules exported at the root level:

```
intrusive-red-black-tree
├── color.rs     -> `Color` enum (u8 repr, RED / BLACK)
├── rb_node.rs   -> `RBNode` core trait abstraction
├── rb_tree.rs   -> `RBTree<T>` container implementation
└── rb_iter.rs   -> `RBTreeIter<T>` in-order iterator
```

### Module Breakdown

| Module | Exports | Responsibilities |
| :--- | :--- | :--- |
| `color` | `Color` | Memory-efficient (`repr(u8)`) enum representing node colors. |
| `rb_node` | `RBNode` | Trait providing accessors/mutators for node pointers, colors, keys, and children. |
| `rb_tree` | `RBTree` | The main container managing tree balance, rotations, lookup, insertion, deletion, and visualization. |
| `rb_iter` | `RBTreeIter` | In-order iterator traversing elements in ascending key order. |

---

## Detailed API Reference

### 1. `RBNode` Trait

To store a structure in an `RBTree`, your type must implement the `RBNode` trait. This decouples your struct's internal layout from the tree's internal balancing mechanisms.

```rust
pub trait RBNode {
    type Node: RBNode;
    type Key: Ord + ?Sized;

    fn set_color(node_ptr: NonNull<Self>, color: Color);
    fn get_color(node_ptr: NonNull<Self>) -> Color;

    fn set_parent(node_ptr: NonNull<Self>, parent: NonNull<Self>);
    fn get_parent(node_ptr: NonNull<Self>) -> NonNull<Self>;

    fn set_child(node_ptr: NonNull<Self>, dir: usize, child: NonNull<Self>);
    fn get_child(node_ptr: NonNull<Self>, dir: usize) -> NonNull<Self>;

    fn get_key<'a>(node_ptr: NonNull<Self>) -> &'a Self::Key;
}
```

### 2. `RBTree<T>` Container

`RBTree<T>` manages the root pointer, sentinel pointer (`nil`), and the cached `minimum` pointer.

#### Key Methods

* `pub fn new(sentinel_node: NonNull<T>) -> Self`
  Constructs a new tree bound to the given sentinel `nil` node.
* `pub fn insert(&mut self, node: NonNull<T>) -> bool`
  Inserts a node into the tree. Returns `false` if an identical key already exists (zero side-effects on duplicate keys).
* `pub fn search<Q>(&self, key: &Q) -> Option<NonNull<T>>`
  Searches for a node matching the borrowed key in $\mathcal{O}(\log N)$ time.
* `pub fn exist<Q>(&self, key: &Q) -> bool`
  Returns `true` if a node matching the borrowed key exists.
* `pub fn delete<Q>(&mut self, key: &Q) -> Option<NonNull<T>>`
  Unlinks and returns the node with the matching key in $\mathcal{O}(\log N)$ time.
* `pub fn delete_min(&mut self) -> Option<NonNull<T>>`
  Pops and returns the leftmost node with the minimum key in $\mathcal{O}(1)$ expected / $\mathcal{O}(\log N)$ worst-case time.
* `pub fn min_node(&self) -> NonNull<T>`
  Returns the cached minimum pointer in $\mathcal{O}(1)$ time.
* `pub fn iter(&self) -> RBTreeIter<T>`
  Creates an in-order iterator over all nodes in ascending key order.

### 3. In-Order Iteration (`RBTreeIter`)

The iterator traverses the tree in ascending key order starting from `tree.min_node()`. Finding the successor of a node takes $\mathcal{O}(1)$ amortized time using parent/child pointer steps without recursion or stack allocations.

---

## Algorithmic Complexity

| Operation | Time Complexity | Space Complexity |
| :--- | :--- | :--- |
| **Search / Lookup** | $\mathcal{O}(\log N)$ | $\mathcal{O}(1)$ |
| **Insertion** | $\mathcal{O}(\log N)$ | $\mathcal{O}(1)$ |
| **Deletion** | $\mathcal{O}(\log N)$ | $\mathcal{O}(1)$ |
| **Pop Minimum** | $\mathcal{O}(\log N)$ worst / $\mathcal{O}(1)$ amortized | $\mathcal{O}(1)$ |
| **Access Minimum** | $\mathcal{O}(1)$ | $\mathcal{O}(1)$ |
| **In-Order Traversal** | $\mathcal{O}(N)$ total / $\mathcal{O}(1)$ amortized per step | $\mathcal{O}(1)$ |

---

## Complete Example

The following self-contained example illustrates defining a Virtual Memory Area (`VmaRegion`) struct, implementing `RBNode`, initializing a tree, performing operations, and cleaning up memory safely.

```rust
use core::ptr::NonNull;
use intrusive_red_black_tree::{Color, RBNode, RBTree};

// 1. Define your domain struct embedding linkage metadata.
#[derive(Debug)]
pub struct VmaRegion {
    pub base_addr: usize,
    pub size: usize,
    pub color: Color,
    pub parent: NonNull<Self>,
    pub children: [NonNull<Self>; 2],
}

impl VmaRegion {
    /// Constructs a heap-allocated (or pool-allocated) node.
    pub fn new(base_addr: usize, size: usize, sentinel: NonNull<Self>) -> NonNull<Self> {
        let boxed = Box::new(Self {
            base_addr,
            size,
            color: Color::RED,
            parent: sentinel,
            children: [sentinel, sentinel],
        });
        NonNull::from(Box::leak(boxed))
    }

    /// Creates a dedicated leaf sentinel (`nil`) node.
    pub fn create_sentinel() -> NonNull<Self> {
        let raw = Box::leak(Box::new(Self {
            base_addr: 0,
            size: 0,
            color: Color::BLACK,
            parent: NonNull::dangling(),
            children: [NonNull::dangling(), NonNull::dangling()],
        }));
        let mut ptr = NonNull::from(raw);
        unsafe {
            ptr.as_mut().parent = ptr;
            ptr.as_mut().children = [ptr, ptr];
        }
        ptr
    }

    /// Free leaked nodes during cleanup.
    pub unsafe fn free(ptr: NonNull<Self>) {
        unsafe { drop(Box::from_raw(ptr.as_ptr())) };
    }
}

// 2. Implement `RBNode` trait bindings.
impl RBNode for VmaRegion {
    type Node = VmaRegion;
    type Key = usize;

    fn set_color(mut node_ptr: NonNull<Self>, color: Color) {
        unsafe { node_ptr.as_mut().color = color };
    }

    fn get_color(node_ptr: NonNull<Self>) -> Color {
        unsafe { node_ptr.as_ref().color }
    }

    fn set_parent(mut node_ptr: NonNull<Self>, parent: NonNull<Self>) {
        unsafe { node_ptr.as_mut().parent = parent };
    }

    fn get_parent(node_ptr: NonNull<Self>) -> NonNull<Self> {
        unsafe { node_ptr.as_ref().parent }
    }

    fn set_child(mut node_ptr: NonNull<Self>, dir: usize, child: NonNull<Self>) {
        unsafe { node_ptr.as_mut().children[dir] = child };
    }

    fn get_child(node_ptr: NonNull<Self>, dir: usize) -> NonNull<Self> {
        unsafe { node_ptr.as_ref().children[dir] }
    }

    fn get_key<'a>(node_ptr: NonNull<Self>) -> &'a Self::Key {
        unsafe { &node_ptr.as_ref().base_addr }
    }
}

fn main() {
    // Setup sentinel node and tree
    let sentinel = VmaRegion::create_sentinel();
    let mut tree = RBTree::new(sentinel);

    // Allocate test nodes
    let n20 = VmaRegion::new(20, 0x1000, sentinel);
    let n10 = VmaRegion::new(10, 0x1000, sentinel);
    let n30 = VmaRegion::new(30, 0x1000, sentinel);
    let n5  = VmaRegion::new(5,  0x1000, sentinel);
    let n15 = VmaRegion::new(15, 0x1000, sentinel);

    // Insert nodes into tree
    assert!(tree.insert(n20));
    assert!(tree.insert(n10));
    assert!(tree.insert(n30));
    assert!(tree.insert(n5));
    assert!(tree.insert(n15));

    // Duplicate insertion check (must return false with zero side effects)
    assert!(!tree.insert(n20));

    // Verify O(1) minimum access
    assert_eq!(tree.min_node(), n5);

    // Perform O(log N) zero-allocation lookup
    assert_eq!(tree.search(&15), Some(n15));
    assert!(tree.exist(&30));
    assert!(!tree.exist(&99));

    // In-order traversal iteration
    let sorted_keys: Vec<usize> = tree.iter().map(|ptr| *VmaRegion::get_key(ptr)).collect();
    assert_eq!(sorted_keys, vec![5, 10, 15, 20, 30]);

    // Unlink nodes
    assert_eq!(tree.delete(&10), Some(n10));
    assert!(!tree.exist(&10));

 
    // Clean up memory
    unsafe {
        VmaRegion::free(n20);
        VmaRegion::free(n10);
        VmaRegion::free(n30);
        VmaRegion::free(n5);
        VmaRegion::free(n15);
        VmaRegion::free(sentinel);
    }
}
```

---

## Safety Guidelines

Because intrusive data structures manipulate raw pointers (`NonNull<T>`):

1. **Address Pinning:** Inserted nodes must remain at stable memory addresses. Do not move or reallocate a node while it is linked in the tree.
2. **Explicit Unlinking:** Always delete or pop a node from the tree before dropping or freeing its underlying memory.
3. **Sentinel Lifetime:** The sentinel node (`nil`) passed to `RBTree::new` must remain initialized and valid throughout the entire life of the tree instance.

---
## ⚖️ License

Licensed under the **MIT License**. See [LICENSE](LICENSE) for details.

[crate-badge]: https://img.shields.io/crates/v/intrusive-red-black-tree.svg
[crate-link]: https://crates.io/crates/intrusive-red-black-tree
[docs-image]: https://docs.rs/intrusive-red-black-tree/badge.svg
[docs-link]: https://docs.rs/intrusive-red-black-tree
[license-image]: https://img.shields.io/badge/MIT-blue.svg
[repo-size]: https://img.shields.io/github/repo-size/KiyanSisakhti/intrusive-red-black-tree
[total-lines]: https://aschey.tech/tokei/github/KiyanSisakhti/intrusive-red-black-tree
[git-ci]:https://github.com/KiyanSisakhti/intrusive-red-black-tree/actions/workflows/rust.yml/badge.svg?branch=main

[license-link]: #license