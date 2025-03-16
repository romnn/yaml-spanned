pub struct Display<'a, T>(pub &'a T);

impl<'a, T> std::fmt::Debug for Display<'a, T>
where
    T: std::fmt::Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}
