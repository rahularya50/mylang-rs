pub fn explore<Node, Tmp, Ret, I: IntoIterator<Item = Node>>(
    root: Node,
    mut pre_recursion: impl FnMut(&mut Node) -> (I, Tmp),
    mut post_recursion: impl FnMut(Node, Tmp, Vec<Ret>) -> Ret,
) -> Ret {
    explore_driver(root, &mut pre_recursion, &mut post_recursion)
}

fn explore_driver<Node, Tmp, Ret, I: IntoIterator<Item = Node>>(
    mut root: Node,
    pre_recursion: &mut impl FnMut(&mut Node) -> (I, Tmp),
    post_recursion: &mut impl FnMut(Node, Tmp, Vec<Ret>) -> Ret,
) -> Ret {
    let (children, tmp) = pre_recursion(&mut root);
    let child_rets = children
        .into_iter()
        .map(|todo| explore_driver(todo, pre_recursion, post_recursion))
        .collect();
    post_recursion(root, tmp, child_rets)
}
