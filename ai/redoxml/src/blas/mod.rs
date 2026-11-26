//! Optimized BLAS Routines
use crate::tensor::{Tensor, TensorType};

/// Initialize BLAS
pub fn init() -> Result<(), &'static str> {
    log::info!("Initializing BLAS");
    Ok(())
}

/// General Matrix Multiply (GEMM)
pub async fn gemm<T: TensorType>(a: &Tensor<T>, b: &Tensor<T>) -> Result<Tensor<T>, &'static str> {
    let a_shape = a.shape();
    let b_shape = b.shape();

    if a_shape.ndim() != 2 || b_shape.ndim() != 2 {
        return Err("GEMM requires 2D tensors");
    }

    let m = a_shape.get_dims()[0];
    let k = a_shape.get_dims()[1];
    let n = b_shape.get_dims()[1];

    if k != b_shape.get_dims()[0] {
        return Err("Incompatible matrix dimensions");
    }

    // For small matrices, use naive
    if m * n * k < 100 * 100 * 100 {
        return gemm_naive(a, b, m, k, n).await;
    }

    // Use tiled implementation for larger matrices
    gemm_tiled(a, b, m, k, n).await
}

async fn gemm_naive<T: TensorType>(
    a: &Tensor<T>,
    b: &Tensor<T>,
    m: usize,
    k: usize,
    n: usize,
) -> Result<Tensor<T>, &'static str> {
    // Need access to raw data. Assuming Tensor exposes it or we add a method.
    // Since we are in the same crate, we might need to make TensorData public or add a method.
    // For now, let's assume we can get a slice or similar.
    // But wait, Tensor struct fields are private. I need to update Tensor to allow access.
    // I will implement this assuming upcoming changes to Tensor.

    let a_data = a.data_as_slice().ok_or("Data not on CPU")?;
    let b_data = b.data_as_slice().ok_or("Data not on CPU")?;
    let mut c_data = vec![T::zero(); m * n];

    for i in 0..m {
        for j in 0..n {
            let mut sum = T::zero();
            for p in 0..k {
                sum = sum + a_data[i * k + p] * b_data[p * n + j];
            }
            c_data[i * n + j] = sum;
        }
    }

    let shape = crate::tensor::Shape::new(vec![m, n]);
    Ok(Tensor::new(shape, c_data))
}

async fn gemm_tiled<T: TensorType>(
    a: &Tensor<T>,
    b: &Tensor<T>,
    m: usize,
    k: usize,
    n: usize,
) -> Result<Tensor<T>, &'static str> {
    let a_data = a.data_as_slice().ok_or("Data not on CPU")?;
    let b_data = b.data_as_slice().ok_or("Data not on CPU")?;
    let mut c_data = vec![T::zero(); m * n];

    const BLOCK_SIZE: usize = 32;

    for i_block in (0..m).step_by(BLOCK_SIZE) {
        let i_end = std::cmp::min(i_block + BLOCK_SIZE, m);
        for j_block in (0..n).step_by(BLOCK_SIZE) {
            let j_end = std::cmp::min(j_block + BLOCK_SIZE, n);
            for p_block in (0..k).step_by(BLOCK_SIZE) {
                let p_end = std::cmp::min(p_block + BLOCK_SIZE, k);

                for i in i_block..i_end {
                    for j in j_block..j_end {
                        let mut sum = T::zero();
                        for p in p_block..p_end {
                            sum = sum + a_data[i * k + p] * b_data[p * n + j];
                        }
                        c_data[i * n + j] = c_data[i * n + j] + sum;
                    }
                }
            }
        }
    }

    let shape = crate::tensor::Shape::new(vec![m, n]);
    Ok(Tensor::new(shape, c_data))
}
