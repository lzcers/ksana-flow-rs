trait Context<T> {
    fn get(&self, key: &str) -> Option<T>;
    fn set(&mut self, key: &str, value: T);
}

trait Node {
    fn execute<V, T: Context<V>>(&self, ctx: &mut T);
}
