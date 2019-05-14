//! Convienence methods on slices.

use std::{
    error::Error,
    fmt::{self, Display, Formatter},
    slice::SliceIndex,
};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash, Default)]
pub(crate) struct BoundsError;

impl Display for BoundsError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.description())
    }
}

impl Error for BoundsError {
    fn description(&self) -> &str {
        "out of bounds"
    }
}

pub(crate) trait SliceExt<T> {
    fn first_res(&self) -> Result<&T, BoundsError>;
    fn first_mut_res(&mut self) -> Result<&mut T, BoundsError>;
    fn split_first_res(&self) -> Result<(&T, &[T]), BoundsError>;
    fn split_first_mut_res(&mut self) -> Result<(&mut T, &mut [T]), BoundsError>;
    fn split_last_res(&self) -> Result<(&T, &[T]), BoundsError>;
    fn split_last_mut_res(&mut self) -> Result<(&mut T, &mut [T]), BoundsError>;
    fn last_res(&self) -> Result<&T, BoundsError>;
    fn last_mut_res(&mut self) -> Result<&mut T, BoundsError>;
    fn get_res<I>(&self, index: I) -> Result<&<I as SliceIndex<[T]>>::Output, BoundsError>
    where
        I: SliceIndex<[T]>;
    fn get_mut_res<I>(
        &mut self,
        index: I,
    ) -> Result<&mut <I as SliceIndex<[T]>>::Output, BoundsError>
    where
        I: SliceIndex<[T]>;
}

impl<T> SliceExt<T> for [T] {
    fn first_res(&self) -> Result<&T, BoundsError> {
        self.first().ok_or(BoundsError)
    }

    fn first_mut_res(&mut self) -> Result<&mut T, BoundsError> {
        self.first_mut().ok_or(BoundsError)
    }

    fn split_first_res(&self) -> Result<(&T, &[T]), BoundsError> {
        self.split_first().ok_or(BoundsError)
    }

    fn split_first_mut_res(&mut self) -> Result<(&mut T, &mut [T]), BoundsError> {
        self.split_first_mut().ok_or(BoundsError)
    }

    fn split_last_res(&self) -> Result<(&T, &[T]), BoundsError> {
        self.split_last().ok_or(BoundsError)
    }

    fn split_last_mut_res(&mut self) -> Result<(&mut T, &mut [T]), BoundsError> {
        self.split_last_mut().ok_or(BoundsError)
    }

    fn last_res(&self) -> Result<&T, BoundsError> {
        self.last().ok_or(BoundsError)
    }

    fn last_mut_res(&mut self) -> Result<&mut T, BoundsError> {
        self.last_mut().ok_or(BoundsError)
    }

    fn get_res<I>(&self, index: I) -> Result<&<I as SliceIndex<[T]>>::Output, BoundsError>
    where
        I: SliceIndex<[T]>,
    {
        self.get(index).ok_or(BoundsError)
    }

    fn get_mut_res<I>(
        &mut self,
        index: I,
    ) -> Result<&mut <I as SliceIndex<[T]>>::Output, BoundsError>
    where
        I: SliceIndex<[T]>,
    {
        self.get_mut(index).ok_or(BoundsError)
    }
}
