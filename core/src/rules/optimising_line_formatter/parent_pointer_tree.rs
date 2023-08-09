use std::cell::{Ref, RefCell, RefMut};
use std::fmt::Debug;

/// Each node in a [`ParentPointerTree`] refers only to its parent.
#[derive(Clone)]
struct Node<T> {
    /// The data held at the given node
    value: T,

    /// The index into the [`ParentPointerTree`]'s data list for this node's
    /// parent.
    parent_idx: Option<usize>,
}

/// A parent pointer tree or cactus/spaghetti stack is a data structure where
/// nodes only specify their parent node. Most usefully, this can represent
/// multiple overlapping and ordered lists.
///
/// Memory use for this data structure can only grow as the links between the
/// nodes must remain stable.
#[derive(Clone)]
pub struct ParentPointerTree<T> {
    /// All the nodes contained within this tree.
    ///
    /// Interior mutability is used to allow [`NodeRef`]s to add successors to
    /// themselves. For this to be possible, each node must be able to retrieve
    /// a mutable reference to their referent list, but holding a mutable
    /// reference would mean these values are difficult to iterate, and
    /// impossible to copy or clone, and would generally make the
    /// borrow-checker a pain to deal with.
    ///
    /// The downside of using interior mutability here is that it's possible for
    /// a call to `add_successors` to panic at runtime, because an existing
    /// borrow exists.
    data: RefCell<Vec<Node<T>>>,
}
impl<T> ParentPointerTree<T> {
    pub fn new(root: T) -> Self {
        Self {
            data: RefCell::new(vec![Node {
                value: root,
                parent_idx: None,
            }]),
        }
    }
    pub fn reserve(&self, capacity: usize) {
        let mut data = self.data.borrow_mut();
        data.reserve(capacity);
    }
    pub fn capacity(&self) -> usize {
        self.data.borrow().capacity()
    }
    pub fn root(&self) -> NodeRef<'_, T> {
        NodeRef {
            list: self,
            index: 0,
        }
    }
    pub fn len(&self) -> usize {
        self.data.borrow().len()
    }
}
impl<T: Debug> Debug for ParentPointerTree<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ParentPointerTree[")?;
        for (index, val) in self.data.borrow().iter().enumerate() {
            if index > 0 {
                write!(f, ", ")?;
            } else {
                write!(f, " ")?;
            }
            write!(
                f,
                "Node {{ index: {index}, val: {:?}, parent: {:?} }}",
                val.value, val.parent_idx
            )?;
        }
        write!(f, "]")
    }
}
impl<'a, T> IntoIterator for &'a ParentPointerTree<T> {
    type Item = NodeRef<'a, T>;
    type IntoIter = PPTIter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        PPTIter {
            index: 0,
            list: self,
        }
    }
}

/// Iterates each node in a [`ParentPointerTree`] in their creation order.
///
/// It is guaranteed that all of a node's parents will have been iterated
/// before reaching the child.
pub struct PPTIter<'list, T> {
    index: usize,
    list: &'list ParentPointerTree<T>,
}
impl<'list, T> Iterator for PPTIter<'list, T> {
    type Item = NodeRef<'list, T>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.list.data.borrow().len() {
            None
        } else {
            let index = self.index;
            self.index += 1;
            Some(NodeRef {
                list: self.list,
                index,
            })
        }
    }
}

/// Used to losely reference a [`Node`] in a [`ParentPointerTree`].
#[derive(Clone, Copy)]
pub struct NodeRef<'list, T> {
    list: &'list ParentPointerTree<T>,
    index: usize,
}
impl<T> NodeRef<'_, T> {
    pub fn index(&self) -> usize {
        self.index
    }
}
impl<T: Debug> Debug for NodeRef<'_, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "NodeRef {{ index: {}, val: {:?} }}",
            self.index,
            self.get()
        )
    }
}
impl<T> PartialEq for NodeRef<'_, T> {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index && std::ptr::eq(self.list, other.list)
    }
}
impl<T> Eq for NodeRef<'_, T> {}

impl<'list, T> NodeRef<'list, T> {
    /// Inserts a node into the [`ParentPointerTree`] with this node as its parent.
    pub fn add_successor(&self, val: T) -> NodeRef<'list, T> {
        let mut data = self.list.data.borrow_mut();
        let index = data.len();
        data.push(Node {
            value: val,
            parent_idx: Some(self.index),
        });
        NodeRef {
            list: self.list,
            index,
        }
    }

    /// Dereference to a [`Ref`] for the underlying data
    /// ### Panics
    /// Panics if the underlying list is currently mutably borrowed.
    pub fn get(&self) -> Ref<'list, T> {
        Ref::map(self.list.data.borrow(), |list| &list[self.index].value)
    }

    /// Dereference to a [`RefMut`] for the underlying data
    /// ### Panics
    /// Panics if the underlying list is currently borrowed.
    pub fn get_mut(&self) -> RefMut<'list, T> {
        RefMut::map(self.list.data.borrow_mut(), |list| {
            &mut list[self.index].value
        })
    }
    pub fn parent(&self) -> Option<NodeRef<'list, T>> {
        self.walk_parents().nth(1)
    }

    /// Iterates from a node through its parents while providing direct access
    /// to their data
    pub fn walk_parents_data(&self) -> impl Iterator<Item = Ref<'list, T>> {
        self.walk_parents().map(|node| node.get())
    }

    /// Iterate from a node through its parent nodes
    pub fn walk_parents(&self) -> impl Iterator<Item = NodeRef<'list, T>> {
        NodeRefIter {
            next_node: Some(self.index),
            tree: self.list,
        }
    }
}

/// An iterator used to iterate from a node through its parent nodes.
struct NodeRefIter<'list, T> {
    next_node: Option<usize>,
    tree: &'list ParentPointerTree<T>,
}
impl<'list, T> Iterator for NodeRefIter<'list, T> {
    type Item = NodeRef<'list, T>;
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(node) = self.next_node {
            self.next_node = self.tree.data.borrow()[node].parent_idx;
            Some(NodeRef {
                list: self.tree,
                index: node,
            })
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use itertools::Itertools;

    use super::*;

    #[test]
    fn ppt_ref_iter() {
        let ppt = ParentPointerTree::new(0);
        let root = ppt.root();
        let trunk1 = root.add_successor(1);
        let branch1 = trunk1.add_successor(2);
        let branch2 = trunk1.add_successor(3);
        let trunk2 = root.add_successor(4);

        assert_eq!(
            ppt.into_iter().collect_vec(),
            vec![root, trunk1, branch1, branch2, trunk2],
            "PPT::ref_iter() should iterate all nodes in the tree"
        );
    }

    #[test]
    fn ppt_len() {
        let ppt = ParentPointerTree::new(0);
        assert_eq!(ppt.len(), 1, "After construction, PPT::len() should be 1");
        let root = ppt.root();

        let trunk = root.add_successor(1);
        assert_eq!(
            ppt.len(),
            2,
            "Adding a successor should increment the len() of a PPT by 1"
        );
        trunk.add_successor(2);
        assert_eq!(
            ppt.len(),
            3,
            "Adding a successor should increment the len() of a PPT by 1"
        );
        trunk.add_successor(3);
        assert_eq!(
            ppt.len(),
            4,
            "Adding a successor should increment the len() of a PPT by 1"
        );
    }

    #[test]
    fn node_ref_iter() {
        let ppt = ParentPointerTree::new(0);
        let root = ppt.root();
        let trunk = root.add_successor(1);
        let branch1 = trunk.add_successor(2);
        let branch11 = branch1.add_successor(4);
        let branch2 = trunk.add_successor(3);
        let branch22 = branch2.add_successor(5);

        assert_eq!(
            branch1.walk_parents().collect_vec(),
            &[branch1, trunk, root]
        );
        assert_eq!(
            branch2.walk_parents().collect_vec(),
            &[branch2, trunk, root]
        );

        assert_eq!(
            branch11.walk_parents().collect_vec(),
            &[branch11, branch1, trunk, root]
        );
        assert_eq!(
            branch22.walk_parents().collect_vec(),
            &[branch22, branch2, trunk, root]
        );
    }

    #[test]
    fn node_eq() {
        let ppt = ParentPointerTree::new(0);
        let root = ppt.root();
        let branch1 = root.add_successor(1).add_successor(2);
        let branch2 = root.add_successor(1).add_successor(2);

        assert_eq!(
            branch1.walk_parents_data().map(|r| *r).collect_vec(),
            &[2, 1, 0]
        );
        assert_eq!(
            branch2.walk_parents_data().map(|r| *r).collect_vec(),
            &[2, 1, 0]
        );

        assert_ne!(
            branch1, branch2,
            "Branches with the same values but different nodes should not be equal"
        );
    }

    #[test]
    fn node_adding_successors() {
        let ppt = ParentPointerTree::new("A");
        /*
            Constructed tree looks is:
                     A
                    / \
                   B   C
                 / | \  \
                D  E  F  G
               / \
              H   I
        */
        let a = ppt.root();
        let b = a.add_successor("B");
        let c = a.add_successor("C");

        let d = b.add_successor("D");
        let e = b.add_successor("E");
        let f = b.add_successor("F");
        let g = c.add_successor("G");

        let h = d.add_successor("H");
        let i = d.add_successor("I");

        fn get_iter_str(node: NodeRef<'_, &str>) -> String {
            node.walk_parents_data().join("")
        }
        assert_eq!(get_iter_str(h), "HDBA");
        assert_eq!(get_iter_str(i), "IDBA");
        assert_eq!(get_iter_str(e), "EBA");
        assert_eq!(get_iter_str(f), "FBA");
        assert_eq!(get_iter_str(g), "GCA");
    }

    #[test]
    fn reserve_capacity() {
        let ppt = ParentPointerTree::new("A");
        let reserve_count = 10;
        ppt.reserve(reserve_count);

        assert!(ppt.capacity() >= reserve_count);
    }
}
