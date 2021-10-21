use core::fmt::Debug;

pub trait StepByOne {
    fn step(&mut self);
}

#[derive(Clone, Copy)]
pub struct ObjectRange<T>
where
    T: StepByOne + Copy + Debug,
{
    start: T,
    end: T,
}

impl<T> ObjectRange<T>
where
    T: StepByOne + Copy + Debug,
{
    pub fn new(start: T, end: T) -> Self {
        Self { start, end }
    }

    pub fn get_start(&self) -> T {
        self.start
    }
    pub fn get_end(&self) -> T {
        self.end
    }
}

impl<T> Debug for ObjectRange<T>
where
    T: StepByOne + Copy + Debug,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ObjectRange")
            .field("start", &self.start)
            .field("end", &self.end)
            .finish()
    }
}

pub struct ObjectRangeIterator<T>
where
    T: StepByOne + Copy + PartialEq,
{
    current: T,
    end: T,
}

impl<T> ObjectRangeIterator<T>
where
    T: StepByOne + Copy + PartialEq,
{
    pub fn new(start: T, end: T) -> Self {
        Self {
            current: start,
            end,
        }
    }
}

impl<T> Iterator for ObjectRangeIterator<T>
where
    T: StepByOne + Copy + PartialEq,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current == self.end {
            None
        } else {
            let t = self.current;
            self.current.step();
            Some(t)
        }
    }
}

impl<T> IntoIterator for ObjectRange<T>
where
    T: StepByOne + Copy + PartialEq + Debug,
{
    type Item = T;
    type IntoIter = ObjectRangeIterator<T>;

    fn into_iter(self) -> Self::IntoIter {
        ObjectRangeIterator::new(self.start, self.end)
    }
}
