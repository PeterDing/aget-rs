pub trait StackLike<T> {
    fn push(&mut self, item: T);

    fn pop(&mut self) -> Option<T>;

    fn len(&self) -> usize;
}
