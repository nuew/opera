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

#[cfg(test)]
mod tests {
    use super::{BoundsError, SliceExt};

    const EMPTY_ARRAY: [bool; 0] = [];
    const SINGLE_ARRAY: [bool; 1] = [true];
    const DOUBLE_ARRAY: [bool; 2] = [true, false];

    #[test]
    fn first_res_empty() {
        assert_eq!(EMPTY_ARRAY.first_res(), Err(BoundsError));
    }

    #[test]
    fn first_res_single() {
        assert_eq!(SINGLE_ARRAY.first_res(), Ok(&true));
    }

    #[test]
    fn first_res_double() {
        assert_eq!(DOUBLE_ARRAY.first_res(), Ok(&true));
    }

    #[test]
    fn first_mut_res_empty() {
        let mut empty_array = EMPTY_ARRAY.clone();
        assert_eq!(empty_array.first_mut_res(), Err(BoundsError));
    }

    #[test]
    fn first_mut_res_single() {
        let mut single_array = SINGLE_ARRAY.clone();
        let first_mut = single_array.first_mut_res().unwrap();
        assert_eq!(first_mut, &true);
        *first_mut = false;
        assert_eq!(single_array, [false]);
    }

    #[test]
    fn first_mut_res_double() {
        let mut double_array = DOUBLE_ARRAY.clone();
        let first_mut = double_array.first_mut_res().unwrap();
        assert_eq!(first_mut, &true);
        *first_mut = false;
        assert_eq!(double_array, [false, false]);
    }

    #[test]
    fn split_first_res_empty() {
        assert_eq!(EMPTY_ARRAY.split_first_res(), Err(BoundsError));
    }

    #[test]
    fn split_first_res_single() {
        assert_eq!(SINGLE_ARRAY.split_first_res(), Ok((&true, [].as_ref())));
    }

    #[test]
    fn split_first_res_double() {
        assert_eq!(
            DOUBLE_ARRAY.split_first_res(),
            Ok((&true, [false].as_ref()))
        );
    }

    #[test]
    fn split_first_mut_res_empty() {
        let mut empty_array = EMPTY_ARRAY.clone();
        assert_eq!(empty_array.split_first_mut_res(), Err(BoundsError));
    }

    #[test]
    fn split_first_mut_res_single() {
        let mut single_array = SINGLE_ARRAY.clone();
        let (car_mut, cdr_mut) = single_array.split_first_mut_res().unwrap();

        assert_eq!(car_mut, &true);
        assert_eq!(cdr_mut, &[]);

        *car_mut = false;
        assert_eq!(single_array, [false]);
    }

    #[test]
    fn split_first_mut_res_double() {
        let mut double_array = DOUBLE_ARRAY.clone();
        let (car_mut, cdr_mut) = double_array.split_first_mut_res().unwrap();

        assert_eq!(car_mut, &true);
        assert_eq!(cdr_mut, &[false]);

        *car_mut = false;
        *cdr_mut.first_mut().unwrap() = true;
        assert_eq!(double_array, [false, true]);
    }

    #[test]
    fn split_last_res_empty() {
        assert_eq!(EMPTY_ARRAY.split_last_res(), Err(BoundsError));
    }

    #[test]
    fn split_last_res_single() {
        assert_eq!(SINGLE_ARRAY.split_last_res(), Ok((&true, [].as_ref())));
    }

    #[test]
    fn split_last_res_double() {
        assert_eq!(DOUBLE_ARRAY.split_last_res(), Ok((&false, [true].as_ref())));
    }

    #[test]
    fn split_last_mut_res_empty() {
        let mut empty_array = EMPTY_ARRAY.clone();
        assert_eq!(empty_array.split_last_mut_res(), Err(BoundsError));
    }

    #[test]
    fn split_last_mut_res_single() {
        let mut single_array = SINGLE_ARRAY.clone();
        let (last_mut, rest_mut) = single_array.split_last_mut_res().unwrap();

        assert_eq!(last_mut, &true);
        assert_eq!(rest_mut, &[]);

        *last_mut = false;
        assert_eq!(single_array, [false]);
    }

    #[test]
    fn split_last_mut_res_double() {
        let mut double_array = DOUBLE_ARRAY.clone();
        let (last_mut, rest_mut) = double_array.split_last_mut_res().unwrap();

        assert_eq!(last_mut, &false);
        assert_eq!(rest_mut, &[true]);

        *last_mut = true;
        *rest_mut.last_mut().unwrap() = false;
        assert_eq!(double_array, [false, true]);
    }

    #[test]
    fn last_res_empty() {
        assert_eq!(EMPTY_ARRAY.last_res(), Err(BoundsError));
    }

    #[test]
    fn last_res_single() {
        assert_eq!(SINGLE_ARRAY.last_res(), Ok(&true));
    }

    #[test]
    fn last_res_double() {
        assert_eq!(DOUBLE_ARRAY.last_res(), Ok(&false));
    }

    #[test]
    fn last_mut_res_empty() {
        let mut empty_array = EMPTY_ARRAY.clone();
        assert_eq!(empty_array.last_mut_res(), Err(BoundsError));
    }

    #[test]
    fn last_mut_res_single() {
        let mut single_array = SINGLE_ARRAY.clone();
        let last_mut = single_array.last_mut_res().unwrap();
        assert_eq!(last_mut, &true);
        *last_mut = false;
        assert_eq!(single_array, [false]);
    }

    #[test]
    fn last_mut_res_double() {
        let mut double_array = DOUBLE_ARRAY.clone();
        let last_mut = double_array.last_mut_res().unwrap();
        assert_eq!(last_mut, &false);
        *last_mut = true;
        assert_eq!(double_array, [true, true]);
    }

    #[test]
    fn get_res_empty() {
        assert_eq!(EMPTY_ARRAY.get_res(usize::min_value()), Err(BoundsError));
        assert_eq!(EMPTY_ARRAY.get_res(usize::max_value()), Err(BoundsError));
    }

    #[test]
    fn get_res_single() {
        assert_eq!(SINGLE_ARRAY.get_res(0), Ok(&true));
        assert_eq!(SINGLE_ARRAY.get_res(1), Err(BoundsError));
        assert_eq!(SINGLE_ARRAY.get_res(usize::max_value()), Err(BoundsError));
    }

    #[test]
    fn get_res_double() {
        assert_eq!(DOUBLE_ARRAY.get_res(0), Ok(&true));
        assert_eq!(DOUBLE_ARRAY.get_res(1), Ok(&false));
        assert_eq!(DOUBLE_ARRAY.get_res(2), Err(BoundsError));
        assert_eq!(DOUBLE_ARRAY.get_res(usize::max_value()), Err(BoundsError));
    }

    #[test]
    fn get_mut_res_empty() {
        let mut empty_array = EMPTY_ARRAY.clone();
        assert_eq!(
            empty_array.get_mut_res(usize::min_value()),
            Err(BoundsError)
        );
        assert_eq!(
            empty_array.get_mut_res(usize::max_value()),
            Err(BoundsError)
        );
    }

    #[test]
    fn get_mut_res_single() {
        let mut single_array = SINGLE_ARRAY.clone();
        let value = single_array.get_mut_res(0).unwrap();

        assert_eq!(value, &true);
        *value = false;
        assert_eq!(single_array, [false]);

        assert_eq!(single_array.get_mut_res(1), Err(BoundsError));
        assert_eq!(
            single_array.get_mut_res(usize::max_value()),
            Err(BoundsError)
        );
    }

    #[test]
    fn get_mut_res_double() {
        let mut double_array = DOUBLE_ARRAY.clone();
        let value = double_array.get_mut_res(0).unwrap();

        assert_eq!(value, &true);
        *value = false;
        assert_eq!(double_array, [false, false]);

        assert_eq!(double_array.get_mut_res(2), Err(BoundsError));
        assert_eq!(
            double_array.get_mut_res(usize::max_value()),
            Err(BoundsError)
        );
    }
}
