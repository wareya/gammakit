// Going to hell and back to avoid using clone().
if is_rotatable_binexpr(node)
{
    assert!(node.children.len() == 3);
    
    let mut hold_right_node = ASTNode::Named(NamedNode{name : "".to_string(), children : Vec::new(), line : 0, position : 0});
    std::mem::swap(node.children.get_mut(2).unwrap(), &mut hold_right_node);
    
    match hold_right_node
    {
        ASTNode::Named(mut rightnode) =>
        {
            if is_rotatable_binexpr(&rightnode)
            {
                assert!(rightnode.children.len() == 3);
                
                let mut hold_node = NamedNode{name : "".to_string(), children : Vec::new(), line : 0, position : 0};
                std::mem::swap(node, &mut hold_node);
                
                // current state:
                // hold_node is the former top node, rightnode is the former right node
                // hold_node's left node is a real node
                // hold_node's right node is a dummy node (i.e. one of the ones constructed inline above)
                // rightnode's right node is a real node
                // rightnode's left node is a real node
                
                // move right node's left to top node's right
                std::mem::swap(&mut hold_node.children.last_mut().unwrap(), &mut rightnode.children.first_mut().unwrap());
                // move top mode into the right node's left
                std::mem::swap(&mut ASTNode::Named(hold_node), &mut rightnode.children.first_mut().unwrap());
                // make the right node the top node
                std::mem::swap(node, &mut rightnode);
            }
            else
            {
                // move back
                std::mem::swap(node.children.get_mut(2).unwrap(), &mut ASTNode::Named(rightnode));
            }
        }
        _ =>
        {
            // move back
            std::mem::swap(node.children.get_mut(2).unwrap(), &mut hold_right_node);
        }
    }
}