use accesskit::{Node as AccessKitNode, NodeId, Role, Tree, TreeUpdate};

use crate::{BaseDocument, Node as BlitzDomNode, local_name};

impl BaseDocument {
    pub fn build_accessibility_tree(&self) -> TreeUpdate {
        let mut nodes = std::collections::HashMap::new();
        let mut window = AccessKitNode::new(Role::Window);

        self.visit(|node_id, node| {
            let parent = node
                .parent
                .and_then(|parent_id| nodes.get_mut(&parent_id))
                .map(|(_, parent)| parent)
                .unwrap_or(&mut window);
            let (id, builder) = self.build_accessibility_node(node, parent);

            nodes.insert(node_id, (id, builder));
        });

        let mut nodes: Vec<_> = nodes
            .into_iter()
            .map(|(_, (id, node))| (id, node))
            .collect();
        nodes.push((NodeId(u64::MAX), window));

        let tree = Tree::new(NodeId(u64::MAX));
        TreeUpdate {
            nodes,
            tree: Some(tree),
            focus: NodeId(self.focus_node_id.map(|id| id as u64).unwrap_or(u64::MAX)),
        }
    }

    fn build_accessibility_node(
        &self,
        node: &BlitzDomNode,
        parent: &mut AccessKitNode,
    ) -> (NodeId, AccessKitNode) {
        let id = NodeId(node.id as u64);

        let mut builder = AccessKitNode::default();
        if node.id == 0 {
            builder.set_role(Role::Window)
        } else if let Some(element_data) = node.element_data() {
            let name = element_data.name.local.to_string();

            let role = match &*name {
                "button" => Role::Button,
                "a" | "link" => Role::Link,
                "div" => Role::GenericContainer,
                "header" => Role::Header,
                "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => Role::Heading,
                "p" => Role::Paragraph,
                "section" => Role::Section,
                "img" => Role::Image,
                "input" => match element_data.attr(local_name!("type")).unwrap_or("text") {
                    "text" | "email" | "password" => Role::TextInput,
                    "number" => Role::NumberInput,
                    "checkbox" => Role::CheckBox,
                    "radio" => Role::RadioButton,
                    "submit" | "button" => Role::Button,
                    _ => Role::TextInput,
                },
                _ => Role::GenericContainer,
            };

            builder.set_role(role);
            builder.set_html_tag(name);
        } else if node.is_text_node() {
            builder.set_role(Role::TextRun);
            builder.set_value(node.text_content());
            parent.push_labelled_by(id)
        }

        parent.push_child(id);

        (id, builder)
    }
}
