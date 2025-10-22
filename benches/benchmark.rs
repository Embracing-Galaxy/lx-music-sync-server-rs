use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use std::collections::HashMap;
use std::ptr;

#[derive(Debug, Clone)]
#[allow(unused)]
struct A {
    b: B,
    payload: Vec<String>,
}

impl A {
    fn id(&self) -> u64 {
        self.b.id
    }

    fn gen_old(id: u64) -> A {
        A {
            b: B {
                id,
                payload: format!("old{id}"),
            },
            payload: vec![],
        }
    }
}

#[derive(Debug, Clone)]
#[allow(unused)]
struct B {
    id: u64,
    payload: String,
}

impl B {
    fn gen_new(id: u64) -> B {
        B {
            id,
            payload: format!("new{id}"),
        }
    }
}

fn map_merge(a_vec: &mut Vec<A>, b_vec: Vec<B>) {
    let mut map: HashMap<u64, B> = b_vec.into_iter().map(|b| (b.id, b)).collect();

    for a in a_vec.iter_mut() {
        if map.is_empty() {
            break;
        }
        if let Some(b) = map.remove(&a.id()) {
            a.b = b;
            if map.is_empty() {
                break;
            }
        }
    }
}

fn sparse_merge(a_vec: &mut Vec<A>, b_vec: Vec<B>) {
    let map: HashMap<u64, usize> = b_vec.iter().enumerate().map(|(i, b)| (b.id, i)).collect();
    let mut b_sparse: Vec<Option<B>> = b_vec.into_iter().map(Some).collect();

    for a in a_vec.iter_mut() {
        if let Some(&idx) = map.get(&a.id()) {
            let b_slot = &mut b_sparse[idx] as *mut Option<B>;
            unsafe {
                a.b = ptr::read(b_slot).unwrap();
                ptr::write(b_slot, None);
            }
        }
    }
}

pub fn bench(c: &mut Criterion) {
    let a_vec = (1..21).map(|i| A::gen_old(i)).collect::<Vec<_>>();
    let b_vec = vec![B::gen_new(2), B::gen_new(3)];
    let data = (a_vec, b_vec);

    c.bench_function("merge with map", |b| {
        b.iter_batched(
            || data.clone(),
            |(mut a_vec, b_vec)| map_merge(&mut a_vec, b_vec),
            BatchSize::SmallInput,
        )
    });
    c.bench_function("merge with sparse vec", |b| {
        b.iter_batched(
            || data.clone(),
            |(mut a_vec, b_vec)| sparse_merge(&mut a_vec, b_vec),
            BatchSize::SmallInput,
        )
    });
}

criterion_group!(benches, bench);
criterion_main!(benches);
