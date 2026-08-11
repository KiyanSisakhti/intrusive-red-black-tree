use crate::{color::Color, rb_iter::RBTreeIter, rb_node::RBNode};
use core::{borrow::Borrow, cmp::Ordering, ptr::NonNull};

/// An intrusive Red-Black Tree backed by raw pointers (`NonNull<T>`).
///
/// Uses a sentinel node (`nil`) to simplify leaf/boundary conditions and avoid
/// null-pointer checks in performance-critical paths.
pub struct RBTree<T>
where
    T: RBNode<Node = T>,
{
    /// Pointer to the root node of the tree, or `nil` if empty.
    pub(crate) root: NonNull<T>,

    /// Sentinel node representing null leaves, the tree's boundary, and empty states.
    pub(crate) nil: NonNull<T>,

    /// Cached pointer to the node with the minimum key for O(1) access.
    pub(crate) minimum: NonNull<T>,
}

impl<T> RBTree<T>
where
    T: RBNode<Node = T>,
{
    /// Constructs a new `RBTree` using a pre-allocated sentinel node.
    ///
    /// The sentinel node (`nil`) is initialized as a black node pointing to itself
    /// for parent and child pointers.
    pub fn new(sentinel_node: NonNull<T>) -> RBTree<T> {
        T::set_color(sentinel_node, Color::BLACK);
        T::set_parent(sentinel_node, sentinel_node);
        T::set_child(sentinel_node, 0, sentinel_node);
        T::set_child(sentinel_node, 1, sentinel_node);

        Self {
            root: sentinel_node,
            nil: sentinel_node,
            minimum: sentinel_node,
        }
    }

    /// Searches for a node matching the given key.
    ///
    /// Returns `Some(NonNull<T>)` if found, or `None` if no matching key exists.
    #[inline]
    pub fn search<Q>(&self, key: &Q) -> Option<NonNull<T>>
    where
        Q: Ord + ?Sized,
        T::Key: Borrow<Q>,
    {
        let mut current = self.root;

        while current != self.nil {
            let cmp = key.cmp(T::get_key(current).borrow());

            match cmp {
                Ordering::Equal => return Some(current),
                Ordering::Less => {
                    current = T::get_child(current, 0); // Move left
                }
                Ordering::Greater => {
                    current = T::get_child(current, 1); // Move right
                }
            }
        }

        None
    }

    /// Checks if a node with the specified key exists in the tree.
    pub fn exist<Q>(&self, key: &Q) -> bool
    where
        Q: Ord + ?Sized,
        T::Key: Borrow<Q>,
    {
        self.search(key).is_some()
    }

    /// Performs a generic directional tree rotation around node `x`.
    ///
    /// # Direction Parameter (`dir`)
    /// * `dir = 0`: Rotates right-child `y` up into `x`'s position (Left Rotation).
    /// * `dir = 1`: Rotates left-child `y` up into `x`'s position (Right Rotation).
    ///
    /// Using bitwise XOR (`dir ^ 1`), opposite subtrees are dynamically bound without duplicate code.
    #[inline(always)]
    fn rotate(&mut self, x: NonNull<T>, dir: usize) {
        let opp = dir ^ 1;
        let y: NonNull<T> = T::get_child(x, opp);

        // Turn y's inner child into x's outer child
        T::set_child(x, opp, T::get_child(y, dir));

        if T::get_child(y, dir) != self.nil {
            T::set_parent(T::get_child(y, dir), x);
        }

        // Link x's parent to y
        let x_parent: NonNull<T> = T::get_parent(x);
        T::set_parent(y, x_parent);

        if x_parent == self.nil {
            self.root = y;
        } else {
            let parent_dir = (T::get_child(x_parent, 1) == x) as usize;
            T::set_child(x_parent, parent_dir, y);
        }

        // Put x on y's inner child side
        T::set_child(y, dir, x);
        T::set_parent(x, y);
    }

    /// Inserts a new node into the Red-Black Tree.
    ///
    /// Returns `true` if inserted successfully, or `false` if a node with an identical key already exists.
    #[inline]
    pub fn insert(&mut self, node: NonNull<T>) -> bool {
        let mut node_y: NonNull<T> = self.nil;
        let mut node_x = self.root;

        // Traverse down to find the insertion point
        while node_x != self.nil {
            node_y = node_x;

            let ord = T::get_key(node).cmp(T::get_key(node_x));
            if ord == Ordering::Equal {
                return false; // Duplicate keys are disallowed
            }

            let dir = (ord == Ordering::Greater) as usize;
            node_x = T::get_child(node_x, dir);
        }

        T::set_parent(node, node_y);

        // Initialize node as a red leaf connected to the nil sentinel
        T::set_color(node, Color::RED);
        T::set_child(node, 0, self.nil);
        T::set_child(node, 1, self.nil);

        // Handle root insertion
        if node_y == self.nil {
            self.root = node;
            self.minimum = node;
            T::set_color(node, Color::BLACK);
            return true;
        }

        // Maintain the cached minimum pointer
        if T::get_key(node) < T::get_key(self.minimum) {
            self.minimum = node
        }

        // Attach node to parent
        let dir = (T::get_key(node) > T::get_key(node_y)) as usize;
        T::set_child(node_y, dir, node);

        // Rebalance to restore Red-Black invariants
        self.fix_insert(node);

        true
    }

    /// Restores Red-Black Tree properties after a node insertion.
    ///
    /// Handles consecutive red nodes (Red-Parent violation) using recoloring
    /// and tree rotations across 3 primary cases.
    #[inline(always)]
    fn fix_insert(&mut self, mut z: NonNull<T>) {
        //
        while T::get_color(T::get_parent(z)) != Color::BLACK {
            let z_parent = T::get_parent(z);
            let gp = T::get_parent(z_parent);

            let dir = (T::get_child(gp, 1) == z_parent) as usize;
            let opp = dir ^ 1; // Uncle Dir
            let u = T::get_child(gp, opp);

            if T::get_color(u) == Color::RED {
                // Case 1: Uncle is RED -> Recolor parent, uncle, and grandparent, then move up
                T::set_color(z_parent, Color::BLACK);
                T::set_color(u, Color::BLACK);
                T::set_color(gp, Color::RED);
                //
                z = gp;
            } else {
                // Case 2: Triangle shape (z is inner child) -> Rotate z's parent to align into line
                if z == T::get_child(z_parent, opp) {
                    z = z_parent;
                    self.rotate(z, dir);
                }

                // Case 3: Line shape (z is outer child) -> Recolor and rotate grandparent
                let current_parent = T::get_parent(z);
                let current_gp = T::get_parent(current_parent);
                T::set_color(current_parent, Color::BLACK);
                T::set_color(current_gp, Color::RED);
                self.rotate(current_gp, opp);
            }
        }

        // Root must always remain BLACK
        T::set_color(self.root, Color::BLACK);
    }

    /// Replaces the subtree rooted at node `u` with the subtree rooted at node `v`.
    #[inline(always)]
    fn rb_transplant(&mut self, u: NonNull<T>, v: NonNull<T>) {
        let u_parent = T::get_parent(u);

        if u_parent == self.nil {
            self.root = v;
        } else {
            let dir = (T::get_child(u_parent, 1) == u) as usize;
            T::set_child(u_parent, dir, v);
        }

        T::set_parent(v, u_parent);
    }

    /// Removes a node with the specified key from the tree.
    ///
    /// Returns `Some(NonNull<T>)` containing the unlinked node if found, or `None` if missing.
    #[inline]
    pub fn delete<Q>(&mut self, key: &Q) -> Option<NonNull<T>>
    where
        Q: Ord + ?Sized,
        T::Key: Borrow<Q>,
    {
        let mut z: NonNull<T> = self.nil;

        let mut current: NonNull<T> = self.root;

        while current != self.nil {
            let ord = key.cmp(T::get_key(current).borrow());

            if ord == Ordering::Equal {
                z = current;
                break;
            }
            let dir = (ord == Ordering::Greater) as usize;
            current = T::get_child(current, dir);
        }

        if z == self.nil {
            return None;
        }

        let x: NonNull<T>;
        let mut y = z;
        let mut y_org_col = T::get_color(y);

        //
        let z_left = T::get_child(z, 0);
        let z_right = T::get_child(z, 1);

        // Case A: Node has no left child -> Replace with right child
        if z_left == self.nil {
            x = z_right;
            self.rb_transplant(z, x);
        }
        // Case B: Node has no right child -> Replace with left child
        else if z_right == self.nil {
            x = z_left;
            self.rb_transplant(z, x);
        }
        // Case C: Node has two children -> Replace with in-order successor (`y`)
        else {
            y = self.minimum(z_right);
            y_org_col = T::get_color(y);
            x = T::get_child(y, 1);

            if T::get_parent(y) == z {
                T::set_parent(x, y);
            } else {
                let y_right = T::get_child(y, 1);
                self.rb_transplant(y, y_right);
                T::set_child(y, 1, z_right);
                T::set_parent(z_right, y);
            }

            self.rb_transplant(z, y);

            T::set_child(y, 0, z_left);
            T::set_parent(z_left, y);
            T::set_color(y, T::get_color(z));
        }

        // If a BLACK node was removed/moved, fix double-black violations
        if y_org_col == Color::BLACK {
            self.delete_fix(x);
        }

        // Update cached minimum pointer if the removed node was the minimum
        if z == self.minimum {
            let z_right = T::get_child(z, 1);
            if z_right != self.nil {
                self.minimum = z_right;
            } else {
                self.minimum = T::get_parent(z);
            }
        }

        Some(z)
    }

    /// Removes and returns the minimum node from the tree in O(1) expected time.
    #[inline]
    pub fn delete_min(&mut self) -> Option<NonNull<T>> {
        let z = self.minimum;
        if z == self.nil {
            return None;
        }

        let z_parent = T::get_parent(z);
        let z_right = T::get_child(z, 1);
        let z_color = T::get_color(z);

        // Transplant right child into minimum's position
        if z_parent == self.nil {
            self.root = z_right;
        } else {
            T::set_child(z_parent, 0, z_right);
        }
        T::set_parent(z_right, z_parent);

        // Adjust minimum pointer and fix tree properties
        if z_color == Color::RED {
            self.minimum = z_parent;
        } else if T::get_color(z_right) == Color::RED {
            T::set_color(z_right, Color::BLACK);
            self.minimum = z_right;
        } else {
            self.minimum = z_parent;
            self.delete_fix(z_right);
        }

        Some(z)
    }

    /// Restores Red-Black invariants after node deletion causes a "double-black" node condition.
    #[inline(always)]
    fn delete_fix(&mut self, mut x: NonNull<T>) {
        let mut s: NonNull<T>;

        while x != self.root && T::get_color(x) == Color::BLACK {
            let mut x_parent = T::get_parent(x);

            let dir = (T::get_child(x_parent, 1) == x) as usize;
            let opp = dir ^ 1; // Sibling direction

            s = T::get_child(x_parent, opp);

            // Case 1: Sibling is RED -> Rotate sibling to make it BLACK
            if T::get_color(s) == Color::RED {
                T::set_color(s, Color::BLACK);
                T::set_color(x_parent, Color::RED);
                self.rotate(x_parent, dir);
                x_parent = T::get_parent(x);
                s = T::get_child(x_parent, opp);
            }

            let s_inner = T::get_child(s, dir);
            let s_outer = T::get_child(s, opp);

            // Case 2: Sibling's children are both BLACK -> Recolor sibling RED and push double-black up
            if T::get_color(s_inner) == Color::BLACK && T::get_color(s_outer) == Color::BLACK {
                T::set_color(s, Color::RED);
                x = x_parent;
            } else {
                // Case 3: Sibling's outer child is BLACK -> Rotate inner child up
                if T::get_color(s_outer) == Color::BLACK {
                    T::set_color(s_inner, Color::BLACK);
                    T::set_color(s, Color::RED);
                    self.rotate(s, opp);
                    s = T::get_child(x_parent, opp);
                }

                // Case 4: Sibling's outer child is RED -> Recolor and rotate parent
                T::set_color(s, T::get_color(x_parent));
                T::set_color(x_parent, Color::BLACK);
                T::set_color(T::get_child(s, opp), Color::BLACK);

                self.rotate(x_parent, dir);
                x = self.root;
            }
        }

        T::set_color(x, Color::BLACK);
    }

    /// Finds the node with the minimum key in the subtree rooted at `node`.
    #[inline(always)]
    fn minimum(&self, mut node: NonNull<T>) -> NonNull<T> {
        while let inner = T::get_child(node, 0)
            && inner != self.nil
        {
            node = inner
        }
        node
    }

    /// Returns the in-order successor of the given node, or `None` if `node` is the maximum.
    #[inline]
    pub fn next_node(&self, node: NonNull<T>) -> Option<NonNull<T>> {
        if node == self.nil {
            return None;
        }

        // If right subtree exists, successor is the minimum of right subtree
        let right = T::get_child(node, 1);
        if right != self.nil {
            return Some(self.minimum(right));
        }

        // Traverse up parent chain until finding a left-child relationship
        let mut curr = node;
        let mut parent = T::get_parent(curr);

        while parent != self.nil && curr == T::get_child(parent, 1) {
            curr = parent;
            parent = T::get_parent(curr);
        }

        if parent == self.nil {
            None
        } else {
            Some(parent)
        }
    }

    /// Creates an in-order iterator over all nodes in the tree.
    #[inline(always)]
    pub fn iter<'a>(&'a self) -> RBTreeIter<'a, T> {
        let cr = if self.minimum == self.nil {
            None
        } else {
            Some(self.minimum)
        };

        RBTreeIter {
            tree: self,
            curr: cr,
        }
    }

    /// Returns the sentinel `nil` node pointer.
    pub fn nil_node(&self) -> NonNull<T> {
        self.nil
    }

    /// Returns the root node pointer.
    pub fn root_node(&self) -> NonNull<T> {
        self.root
    }

    /// Returns the cached minimum node pointer.
    pub fn min_node(&self) -> NonNull<T> {
        self.minimum
    }
}
