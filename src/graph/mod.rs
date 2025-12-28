trait Context {
    fn get(&self, key: &str) -> Option<&dyn std::any::Any>;
    fn set(&mut self, key: &str, value: Box<dyn std::any::Any>);
}

trait Node {
    fn execute<T: Context>(&self, ctx: &mut T);
}
