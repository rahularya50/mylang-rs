pub fn explore<Node, Ret, I: IntoIterator<Item = Node>>(
    root: Node,
    mut pre_recursion: impl FnMut(&mut Node) -> I,
    mut post_recursion: impl FnMut(Node, Vec<Ret>) -> Ret,
) -> Ret {
    explore_driver(root, &mut pre_recursion, &mut post_recursion)
}

fn explore_driver<Node, Ret, I: IntoIterator<Item = Node>>(
    mut root: Node,
    pre_recursion: &mut impl FnMut(&mut Node) -> I,
    post_recursion: &mut impl FnMut(Node, Vec<Ret>) -> Ret,
) -> Ret {
    let child_rets = pre_recursion(&mut root)
        .into_iter()
        .map(|todo| explore_driver(todo, pre_recursion, post_recursion))
        .collect();
    post_recursion(root, child_rets)
}
