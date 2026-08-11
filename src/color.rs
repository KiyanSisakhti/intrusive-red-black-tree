//! Represents the color of a node in a Red-Black Tree.
//!
//! Every node must be strictly [`Color::RED`] or [`Color::BLACK`] to enforce
//! the Red-Black tree balancing invariants. Represented as `u8` for minimum memory footprint.

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub enum Color {
    // Indicates a Red node. Red nodes cannot have Red children.
    RED,
    // Indicates a Black node. Used for tree balance and leaf sentinels (`nil`).
    BLACK,
}
