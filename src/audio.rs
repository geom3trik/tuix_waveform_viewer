


pub struct Audio {
    data: Vec<f32>,
}

impl Audio {
    pub fn new(buffer: &[f32]) -> Self {
        let mut data = vec![0.0; buffer.len() * 2];
        data.splice(buffer.len()..data.len(), buffer.iter().cloned());

        

        Self { data }
    }
}



// use std::iter::Iterator;
// #[derive(Debug)]
// pub struct Deinterleaved<'a, T: 'a> {
//     left_buffer: &'a mut [T],
//     right_buffer: &'a mut [T],
//     current: usize,
// }

// impl<'a, T: 'a> Deinterleaved<'a,T> {
//     #[inline]
//     pub fn new(buffer: &'a mut [T]) -> Self {
//         let (left_buffer, right_buffer) = buffer.split_at_mut(buffer.len()/2);
//         Self { left_buffer, right_buffer, current: 0 }
//     }
// }

// impl<'a, T> Iterator for Deinterleaved<'a, T> {
//     type Item = (&'a mut T, &'a mut T);

//     fn next<'b>(&'b mut self) -> Option<Self::Item> {
//         if self.left_buffer.is_empty() || self.current == self.left_buffer.len() {
//             None
//         } else {


//             let tmp_left = std::mem::replace(&mut self.left_buffer, &mut []);
//             let left = &mut tmp_left[self.current];
//             let tmp_right = std::mem::replace(&mut self.left_buffer, &mut []);
//             let right = &mut tmp_right[self.current];

//             self.left_buffer = tmp_left;
//             self.right_buffer = tmp_right;

//             self.current += 1;

//             Some((left, left))
//         }
//     }
// }
// pub trait DeinterleavedIterator<'a, T> {
//     fn deinterleave(&'a mut self) -> Deinterleaved<'a, T>;
// }

// impl<'a, T: 'a> DeinterleavedIterator<'a,T> for [T] {
//     fn deinterleave(&'a mut self) -> Deinterleaved<'a, T> {
//         Deinterleaved::new(self)
//     }
// }

