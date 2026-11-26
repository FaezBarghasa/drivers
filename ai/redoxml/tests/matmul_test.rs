use redoxml::tensor::{Tensor, Shape};
use redoxml::init;

#[tokio::test]
async fn test_matmul_identity() {
    let _ = init();
    
    let size = 32;
    let shape = Shape::new(vec![size, size]);
    
    // A = Identity
    let mut a_data = vec![0.0f32; size * size];
    for i in 0..size {
        a_data[i * size + i] = 1.0;
    }
    let a = Tensor::new(shape.clone(), a_data);
    
    // B = Random-ish (filled with index)
    let mut b_data = vec![0.0f32; size * size];
    for i in 0..size * size {
        b_data[i] = i as f32;
    }
    let b = Tensor::new(shape.clone(), b_data.clone());
    
    // C = A * B = I * B = B
    let c = a.matmul(&b).await.expect("Matmul failed");
    
    let c_slice = c.data_as_slice().expect("Result not on CPU");
    
    for i in 0..size * size {
        assert!((c_slice[i] - b_data[i]).abs() < 1e-5, "Mismatch at index {}", i);
    }
}

#[tokio::test]
async fn test_matmul_tiled_trigger() {
    let _ = init();
    
    // 128x128x128 > 1,000,000 ops, should trigger tiled path
    let size = 128;
    let shape = Shape::new(vec![size, size]);
    
    // A = Identity
    let mut a_data = vec![0.0f32; size * size];
    for i in 0..size {
        a_data[i * size + i] = 1.0;
    }
    let a = Tensor::new(shape.clone(), a_data);
    
    // B = Value 2.0 everywhere
    let b_data = vec![2.0f32; size * size];
    let b = Tensor::new(shape.clone(), b_data);
    
    // C = A * B = B
    let c = a.matmul(&b).await.expect("Matmul failed");
    
    let c_slice = c.data_as_slice().expect("Result not on CPU");
    
    for i in 0..size * size {
        assert!((c_slice[i] - 2.0).abs() < 1e-5, "Mismatch at index {}", i);
    }
}
