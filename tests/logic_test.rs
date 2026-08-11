use bolero::{TypeGenerator, check};
use intrusive_red_black_tree::{Color, RBNode, RBTree};
use rand::seq::SliceRandom;

use core::ptr::NonNull;
use std::boxed::Box;
use std::collections::HashMap;
use std::fmt::Debug;
use std::vec;
use std::vec::Vec;

#[derive(Debug)]
pub struct MyNode {
    key: i32,
    color: Color,
    parent: NonNull<MyNode>,
    children: [NonNull<MyNode>; 2],
}

impl MyNode {
    fn new(key: i32, sentinel: NonNull<Self>) -> NonNull<Self> {
        NonNull::from(Box::leak(Box::new(Self {
            key,
            color: Color::RED,
            parent: sentinel,
            children: [sentinel, sentinel],
        })))
    }

    fn create_sentinel() -> NonNull<Self> {
        let sentinel_raw = Box::leak(Box::new(Self {
            key: 0,
            color: Color::BLACK,
            parent: NonNull::dangling(),
            children: [NonNull::dangling(), NonNull::dangling()],
        }));
        let mut ptr = NonNull::from(sentinel_raw);
        unsafe {
            ptr.as_mut().parent = ptr;
            ptr.as_mut().children = [ptr, ptr];
        }
        ptr
    }

    unsafe fn free(ptr: NonNull<Self>) {
        unsafe { drop(Box::from_raw(ptr.as_ptr())) };
    }
}

impl RBNode for MyNode {
    type Node = MyNode;
    type Key = i32;

    fn set_color(mut node_ptr: NonNull<Self>, color: Color) {
        unsafe { node_ptr.as_mut() }.color = color;
    }

    fn get_color(node_ptr: NonNull<Self>) -> Color {
        unsafe { node_ptr.as_ref() }.color
    }

    fn set_parent(mut node_ptr: NonNull<Self>, parent: NonNull<Self>) {
        unsafe { node_ptr.as_mut() }.parent = parent;
    }

    fn get_parent(node_ptr: NonNull<Self>) -> NonNull<Self> {
        unsafe { node_ptr.as_ref() }.parent
    }

    fn set_child(mut node_ptr: NonNull<Self>, dir: usize, child: NonNull<Self>) {
        unsafe { node_ptr.as_mut() }.children[dir] = child;
    }

    fn get_child(node_ptr: NonNull<Self>, dir: usize) -> NonNull<Self> {
        unsafe { node_ptr.as_ref() }.children[dir]
    }

    fn get_key<'a>(node_ptr: NonNull<Self>) -> &'a Self::Key {
        unsafe { &node_ptr.as_ref().key }
    }
}

// --- Deep RB-Tree Invariants Validation Helpers ---

/// Recursively validates BST properties and Red-Black Tree invariants.
/// Returns (node_count, black_height).
fn check_invariants_rec(
    tree: &RBTree<MyNode>,
    node: NonNull<MyNode>,
    min_bound: Option<i32>,
    max_bound: Option<i32>,
) -> (usize, usize) {
    if node == tree.nil_node() {
        // Nil leaves must always be BLACK
        assert_eq!(MyNode::get_color(node), Color::BLACK);
        return (0, 1); // Sentinel counts as 1 black node in the path height
    }

    let current_key = *MyNode::get_key(node);
    let color = MyNode::get_color(node);

    // 1. Strict BST bound checking
    if let Some(min) = min_bound {
        assert!(
            current_key > min,
            "BST Violation: {} <= {}",
            current_key,
            min
        );
    }
    if let Some(max) = max_bound {
        assert!(
            current_key < max,
            "BST Violation: {} >= {}",
            current_key,
            max
        );
    }

    let left = MyNode::get_child(node, 0);
    let right = MyNode::get_child(node, 1);

    // 2. Parent-Child Pointer Reciprocity
    if left != tree.nil_node() {
        assert_eq!(
            MyNode::get_parent(left),
            node,
            "Left child parent mismatch at key {}",
            current_key
        );
    }
    if right != tree.nil_node() {
        assert_eq!(
            MyNode::get_parent(right),
            node,
            "Right child parent mismatch at key {}",
            current_key
        );
    }

    // 3. Red Property: Red nodes must not have Red children (No double red)
    if color == Color::RED {
        if left != tree.nil_node() {
            assert_ne!(
                MyNode::get_color(left),
                Color::RED,
                "Red-Red violation at key {} -> Left Child {}",
                current_key,
                unsafe { left.as_ref() }.key
            );
        }
        if right != tree.nil_node() {
            assert_ne!(
                MyNode::get_color(right),
                Color::RED,
                "Red-Red violation at key {} -> Right Child {}",
                current_key,
                unsafe { right.as_ref() }.key
            );
        }
    }

    let (left_count, left_bh) = check_invariants_rec(tree, left, min_bound, Some(current_key));
    let (right_count, right_bh) = check_invariants_rec(tree, right, Some(current_key), max_bound);

    // 4. Black Height Uniformity Rule: Every path from node to descendant leaves must have same black height
    assert_eq!(
        left_bh, right_bh,
        "Black-Height mismatch at node {}: left_bh={}, right_bh={}",
        current_key, left_bh, right_bh
    );

    let my_bh = left_bh + if color == Color::BLACK { 1 } else { 0 };

    (left_count + right_count + 1, my_bh)
}

/// Full validation suite for the tree structure and minimum pointer accuracy.
fn check_tree_invariants(tree: &RBTree<MyNode>) -> usize {
    // Root Property: Root must be BLACK
    assert_eq!(
        MyNode::get_color(tree.root_node()),
        Color::BLACK,
        "Root node is not BLACK"
    );

    let (count, _) = check_invariants_rec(tree, tree.root_node(), None, None);

    // Verify Minimum Pointer Consistency
    if count == 0 {
        assert_eq!(
            tree.min_node(),
            tree.nil_node(),
            "Empty tree minimum should be NIL"
        );
    } else {
        let mut curr = tree.root_node();
        while MyNode::get_child(curr, 0) != tree.nil_node() {
            curr = MyNode::get_child(curr, 0);
        }
        assert_eq!(
            tree.min_node(),
            curr,
            "Tree minimum pointer doesn't match actual leftmost node"
        );
    }

    count
}

fn collect_in_order(tree: &RBTree<MyNode>, node: NonNull<MyNode>, vec: &mut Vec<i32>) {
    if node != tree.nil_node() {
        let left = MyNode::get_child(node, 0);
        let right = MyNode::get_child(node, 1);

        collect_in_order(tree, left, vec);
        vec.push(*MyNode::get_key(node));
        collect_in_order(tree, right, vec);
    }
}

// --- Bolero Fuzz Operations ---

#[derive(Debug, Clone, TypeGenerator)]
enum TreeOp {
    Insert(i32),
    Delete(i32),
    Search(i32),
}

#[test]
fn fuzz_differential_workload() {
    check!().with_type::<Vec<TreeOp>>().for_each(|operations| {
        let sentinel = MyNode::create_sentinel();
        let mut tree = RBTree::new(sentinel);
        let mut reference_map = HashMap::new();

        for op in operations {
            match *op {
                TreeOp::Insert(key) => {
                    if !reference_map.contains_key(&key) {
                        let node_ptr = MyNode::new(key, sentinel);
                        reference_map.insert(key, node_ptr);
                        let inserted = tree.insert(node_ptr);
                        assert!(inserted);
                    } else {
                        // Test duplicate insertion handling
                        let temp_node = MyNode::new(key, sentinel);
                        let inserted = tree.insert(temp_node);
                        assert!(!inserted);
                        unsafe {
                            MyNode::free(temp_node);
                        }
                    }
                }
                TreeOp::Delete(key) => {
                    let deleted_node = tree.delete(&key);
                    if let Some(node_ptr) = deleted_node {
                        assert_eq!(*MyNode::get_key(node_ptr), key);
                        reference_map.remove(&key);
                        unsafe {
                            MyNode::free(node_ptr);
                        }
                    } else {
                        assert!(!reference_map.contains_key(&key));
                    }
                }
                TreeOp::Search(key) => {
                    let expect_exist = reference_map.contains_key(&key);
                    assert_eq!(tree.exist(&key), expect_exist);

                    if let Some(res) = tree.search(&key) {
                        assert_eq!(*MyNode::get_key(res), key);
                    }
                }
            }

            // Verification step per operation
            let actual_count = check_tree_invariants(&tree);
            assert_eq!(actual_count, reference_map.len());
        }

        // In-order traversal / Sorting validation check
        let mut tree_keys = Vec::new();
        collect_in_order(&tree, tree.root_node(), &mut tree_keys);
        let mut expected_keys: Vec<i32> = reference_map.keys().copied().collect();
        expected_keys.sort();
        assert_eq!(tree_keys, expected_keys);

        // Iterator check vs In-order
        let iter_keys: Vec<i32> = tree.iter().map(|n| *MyNode::get_key(n)).collect();
        assert_eq!(iter_keys, expected_keys);

        // Safe memory deallocation
        for (_, node_ptr) in reference_map {
            unsafe {
                MyNode::free(node_ptr);
            }
        }
        unsafe {
            MyNode::free(sentinel);
        }
    });
}

#[test]
fn fuzz_bulk_inserts_then_clean_deletions() {
    check!().with_type::<Vec<i32>>().for_each(|keys| {
        let sentinel = MyNode::create_sentinel();
        let mut tree = RBTree::new(sentinel);
        let mut unique_tracker = HashMap::new();

        for &key in keys {
            if !unique_tracker.contains_key(&key) {
                let p = MyNode::new(key, sentinel);
                unique_tracker.insert(key, p);
                tree.insert(p);
            }
        }

        let count = check_tree_invariants(&tree);
        assert_eq!(count, unique_tracker.len());

        for &key in keys {
            if unique_tracker.remove(&key).is_some() {
                let deleted = tree.delete(&key);
                assert!(deleted.is_some());
                unsafe {
                    MyNode::free(deleted.unwrap());
                }
                check_tree_invariants(&tree);
            }
        }

        assert_eq!(tree.root_node(), tree.nil_node());
        assert_eq!(tree.min_node(), tree.nil_node());

        unsafe {
            MyNode::free(sentinel);
        }
    });
}

#[test]
fn fuzz_extreme_sequential_and_duplicates() {
    check!()
        .with_type::<Vec<u8>>() // Dense range (0-255) triggers maximum rotations and color fixes
        .for_each(|small_ints| {
            let sentinel = MyNode::create_sentinel();
            let mut tree = RBTree::new(sentinel);
            let mut active_nodes = HashMap::new();

            for &val in small_ints {
                let key = val as i32;
                if !active_nodes.contains_key(&key) {
                    let p = MyNode::new(key, sentinel);
                    active_nodes.insert(key, p);
                    tree.insert(p);
                } else if let Some(old_p) = tree.delete(&key) {
                    unsafe {
                        MyNode::free(old_p);
                    }
                    let new_p = MyNode::new(key, sentinel);
                    active_nodes.insert(key, new_p);
                    tree.insert(new_p);
                }
            }

            check_tree_invariants(&tree);

            for (_, ptr) in active_nodes {
                unsafe {
                    MyNode::free(ptr);
                }
            }
            unsafe {
                MyNode::free(sentinel);
            }
        });
}

#[test]
fn test_string_key_with_borrow_str() {
    use std::string::String;

    #[derive(Debug)]
    struct StringNode {
        key: String,
        color: Color,
        parent: NonNull<StringNode>,
        children: [NonNull<StringNode>; 2],
    }

    impl StringNode {
        fn new(key: &str, sentinel: NonNull<Self>) -> NonNull<Self> {
            NonNull::from(Box::leak(Box::new(Self {
                key: String::from(key),
                color: Color::RED,
                parent: sentinel,
                children: [sentinel, sentinel],
            })))
        }

        fn create_sentinel() -> NonNull<Self> {
            let sentinel_raw = Box::leak(Box::new(Self {
                key: String::new(),
                color: Color::BLACK,
                parent: NonNull::dangling(),
                children: [NonNull::dangling(), NonNull::dangling()],
            }));
            let mut ptr = NonNull::from(sentinel_raw);
            unsafe {
                ptr.as_mut().parent = ptr;
                ptr.as_mut().children = [ptr, ptr];
            }
            ptr
        }

        unsafe fn free(ptr: NonNull<Self>) {
            unsafe { drop(Box::from_raw(ptr.as_ptr())) };
        }
    }

    impl RBNode for StringNode {
        type Node = StringNode;
        type Key = String;

        fn set_color(mut node_ptr: NonNull<Self>, color: Color) {
            unsafe { node_ptr.as_mut() }.color = color;
        }
        fn get_color(node_ptr: NonNull<Self>) -> Color {
            unsafe { node_ptr.as_ref() }.color
        }
        fn set_parent(mut node_ptr: NonNull<Self>, parent: NonNull<Self>) {
            unsafe { node_ptr.as_mut() }.parent = parent;
        }
        fn get_parent(node_ptr: NonNull<Self>) -> NonNull<Self> {
            unsafe { node_ptr.as_ref() }.parent
        }
        fn set_child(mut node_ptr: NonNull<Self>, dir: usize, child: NonNull<Self>) {
            unsafe { node_ptr.as_mut() }.children[dir] = child;
        }
        fn get_child(node_ptr: NonNull<Self>, dir: usize) -> NonNull<Self> {
            unsafe { node_ptr.as_ref() }.children[dir]
        }
        fn get_key<'a>(node_ptr: NonNull<Self>) -> &'a Self::Key {
            unsafe { &node_ptr.as_ref().key }
        }
    }

    let sentinel = StringNode::create_sentinel();
    let mut tree = RBTree::new(sentinel);
    let node1 = StringNode::new("kernel_core", sentinel);
    let node2 = StringNode::new("allocator", sentinel);

    tree.insert(node1);
    tree.insert(node2);

    // 1. Zero-allocation Search using &str directly
    assert!(tree.exist("kernel_core"));
    assert!(tree.exist("allocator"));
    assert!(!tree.exist("missing_key"));

    let found = tree.search("kernel_core");
    assert!(found.is_some());
    assert_eq!(StringNode::get_key(found.unwrap()), "kernel_core");

    // 2. Delete using &str lookup
    let deleted = tree.delete("allocator");
    assert!(deleted.is_some());
    assert_eq!(StringNode::get_key(deleted.unwrap()), "allocator");
    assert!(!tree.exist("allocator"));

    // Cleanup memory
    unsafe {
        StringNode::free(node1);
        StringNode::free(deleted.unwrap());
        StringNode::free(sentinel);
    }
}

#[test]
fn test_delete_min_basic() {
    let sentinel = MyNode::create_sentinel();
    let mut tree = RBTree::new(sentinel);

    assert!(tree.delete_min().is_none());

    let keys = vec![15, 10, 20, 5, 12, 25, 3];
    for &k in &keys {
        tree.insert(MyNode::new(k, sentinel));
    }

    check_tree_invariants(&tree);

    let mut deleted_keys = Vec::new();
    while let Some(node_ptr) = tree.delete_min() {
        let key = *MyNode::get_key(node_ptr);
        deleted_keys.push(key);

        unsafe {
            MyNode::free(node_ptr);
        }

        check_tree_invariants(&tree);
    }

    assert_eq!(deleted_keys, vec![3, 5, 10, 12, 15, 20, 25]);
    assert_eq!(tree.root_node(), tree.nil_node());

    unsafe {
        MyNode::free(sentinel);
    }
}

#[test]
fn fuzz_delete_min_ordering() {
    check!().with_type::<Vec<i32>>().for_each(|keys| {
        let sentinel = MyNode::create_sentinel();
        let mut tree = RBTree::new(sentinel);
        let mut inserted_keys = HashMap::new();

        for &key in keys {
            if !inserted_keys.contains_key(&key) {
                let node_ptr = MyNode::new(key, sentinel);
                inserted_keys.insert(key, node_ptr);
                tree.insert(node_ptr);
            }
        }

        let mut expected_sorted: Vec<i32> = inserted_keys.keys().copied().collect();
        expected_sorted.sort_unstable();

        let mut actual_popped = Vec::new();

        while let Some(min_ptr) = tree.delete_min() {
            let key = *MyNode::get_key(min_ptr);
            actual_popped.push(key);

            unsafe {
                MyNode::free(min_ptr);
            }

            check_tree_invariants(&tree);
        }

        assert_eq!(actual_popped, expected_sorted);
        assert_eq!(tree.root_node(), tree.nil_node());

        unsafe {
            MyNode::free(sentinel);
        }
    });
}

#[test]
fn visual_test() {
    let sentinel = MyNode::create_sentinel();
    let mut rb_tree = RBTree::new(sentinel);
    let mut numbers: Vec<i32> = (1..=200).collect();

    let mut rng = rand::rng();

    numbers.shuffle(&mut rng);

    let keys: Vec<i32> = numbers.into_iter().take(30).collect();

    for i in &keys {
        rb_tree.insert(MyNode::new(*i, sentinel));
    }

    println!("");
    println!("");
    println!("Inserted Keys : {:?}", keys);
    println!("");
    println!("");
    print_tree(&rb_tree);

    let rm_keys = &keys[0..keys.len() / 2];

    for r in rm_keys {
        let d = rb_tree.delete(r).unwrap();
        unsafe { MyNode::free(d) };
    }

    println!("");
    println!("");
    println!("Deleted Keys : {:?}", rm_keys);
    println!("");
    println!("");
    print_tree(&rb_tree);
    println!("");
    println!("");

    while let Some(h) = rb_tree.delete_min() {
        unsafe { MyNode::free(h) };
    }
    unsafe { MyNode::free(sentinel) };
}

pub fn print_tree<T: RBNode<Node = T, Key: Debug>>(tree: &RBTree<T>) {
    if tree.root_node() == tree.nil_node() {
        println!("<Empty Tree>");
        return;
    }
    print_node(tree, tree.root_node(), "", true);
}

fn print_node<T: RBNode<Node = T, Key: Debug>>(
    tree: &RBTree<T>,
    node: NonNull<T::Node>,
    prefix: &str,
    is_left: bool,
) {
    if node == tree.nil_node() {
        return;
    }

    let right = T::get_child(node, 1);
    if right != tree.nil_node() {
        let new_prefix = format!("{}{}", prefix, if is_left { "│   " } else { "    " });
        print_node(tree, right, &new_prefix, false);
    }

    let color_str = match T::get_color(node) {
        Color::RED => "\x1b[31mR\x1b[0m",
        Color::BLACK => "\x1b[30mB\x1b[0m",
    };

    let branch = if node == tree.root_node() {
        "── "
    } else if is_left {
        "└── "
    } else {
        "┌── "
    };

    println!("{}{}[{}] {:?}", prefix, branch, color_str, T::get_key(node));

    let left = T::get_child(node, 0);
    if left != tree.nil_node() {
        let new_prefix = format!("{}{}", prefix, if is_left { "    " } else { "│   " });
        print_node(tree, left, &new_prefix, true);
    }
}
