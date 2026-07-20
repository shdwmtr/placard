#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(u32);

impl NodeId {
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

#[derive(Debug, Clone)]
pub enum NodeData {
    Document,
    Element {
        tag: String,
        attrs: Vec<(String, String)>,
    },
    Text(String),
}

#[derive(Debug)]
struct Node {
    parent: Option<NodeId>,
    first_child: Option<NodeId>,
    last_child: Option<NodeId>,
    next_sibling: Option<NodeId>,
    prev_sibling: Option<NodeId>,
    data: NodeData,
}

pub struct Dom {
    nodes: Vec<Node>,
}

impl Dom {
    pub(crate) fn new() -> Self {
        let root = Node {
            parent: None,
            first_child: None,
            last_child: None,
            next_sibling: None,
            prev_sibling: None,
            data: NodeData::Document,
        };
        Self { nodes: vec![root] }
    }

    pub fn root(&self) -> NodeId {
        NodeId(0)
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    fn push_node(&mut self, data: NodeData, parent: NodeId) -> NodeId {
        let id = NodeId(self.nodes.len() as u32);
        self.nodes.push(Node {
            parent: Some(parent),
            first_child: None,
            last_child: None,
            next_sibling: None,
            prev_sibling: None,
            data,
        });
        self.append_child_link(parent, id);
        id
    }

    fn append_child_link(&mut self, parent: NodeId, child: NodeId) {
        let last_child = self.nodes[parent.0 as usize].last_child;
        if let Some(last) = last_child {
            self.nodes[last.0 as usize].next_sibling = Some(child);
            self.nodes[child.0 as usize].prev_sibling = Some(last);
        } else {
            self.nodes[parent.0 as usize].first_child = Some(child);
        }
        self.nodes[parent.0 as usize].last_child = Some(child);
    }

    pub(crate) fn append_element(
        &mut self,
        parent: NodeId,
        tag: &str,
        attrs: Vec<(String, String)>,
    ) -> NodeId {
        self.push_node(
            NodeData::Element {
                tag: tag.to_string(),
                attrs,
            },
            parent,
        )
    }

    pub(crate) fn append_text(&mut self, parent: NodeId, text: &str) -> NodeId {
        self.push_node(NodeData::Text(text.to_string()), parent)
    }

    pub fn set_text_content(&mut self, id: NodeId, text: &str) {
        self.nodes[id.0 as usize].first_child = None;
        self.nodes[id.0 as usize].last_child = None;
        self.append_text(id, text);
    }

    pub fn data(&self, id: NodeId) -> &NodeData {
        &self.nodes[id.0 as usize].data
    }

    pub fn tag(&self, id: NodeId) -> Option<&str> {
        match &self.nodes[id.0 as usize].data {
            NodeData::Element { tag, .. } => Some(tag),
            _ => None,
        }
    }

    pub fn attrs(&self, id: NodeId) -> &[(String, String)] {
        match &self.nodes[id.0 as usize].data {
            NodeData::Element { attrs, .. } => attrs,
            _ => &[],
        }
    }

    pub fn attr(&self, id: NodeId, name: &str) -> Option<&str> {
        self.attrs(id)
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }

    pub fn text(&self, id: NodeId) -> Option<&str> {
        match &self.nodes[id.0 as usize].data {
            NodeData::Text(t) => Some(t),
            _ => None,
        }
    }

    pub fn parent(&self, id: NodeId) -> Option<NodeId> {
        self.nodes[id.0 as usize].parent
    }

    pub fn first_child(&self, id: NodeId) -> Option<NodeId> {
        self.nodes[id.0 as usize].first_child
    }

    pub fn next_sibling(&self, id: NodeId) -> Option<NodeId> {
        self.nodes[id.0 as usize].next_sibling
    }

    pub fn prev_sibling(&self, id: NodeId) -> Option<NodeId> {
        self.nodes[id.0 as usize].prev_sibling
    }

    pub fn children(&self, id: NodeId) -> Children<'_> {
        Children {
            dom: self,
            next: self.first_child(id),
        }
    }
}

pub struct Children<'a> {
    dom: &'a Dom,
    next: Option<NodeId>,
}

impl<'a> Iterator for Children<'a> {
    type Item = NodeId;

    fn next(&mut self) -> Option<NodeId> {
        let cur = self.next?;
        self.next = self.dom.next_sibling(cur);
        Some(cur)
    }
}
