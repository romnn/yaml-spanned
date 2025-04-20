pub struct Display<'a, T>(pub &'a T);

impl<T> std::fmt::Debug for Display<'_, T>
where
    T: std::fmt::Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}
