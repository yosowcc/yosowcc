use std::{collections::BTreeMap};

use ark_ff::{Field};
use ark_std::rand::Rng;
/// Symmetric bi-variate polynomial
#[derive(Debug)]
pub struct SymBVPoly<F: Field> {
    pub coeffs: BTreeMap<(u64, u64), F>,
    pub degree: u64,
}

pub struct Poly<F: Field> {
    pub coeffs: Vec<F>,
    pub degree: u64,
}
impl<F: Field> SymBVPoly<F> {
    pub fn rand<R: Rng>(d: u64, rng: &mut R) -> SymBVPoly<F> {
        let mut coeffs: BTreeMap<(u64, u64), F> = term_powers_for_degree(d)
        .filter(|(dx, dy)| dx >= dy)
        .map(|(dx, dy)| ((dx, dy), F::rand(rng)))
        .collect();

        SymBVPoly {
            coeffs: coeffs,
            degree: d,
        }
    }

    pub fn rand_new<R: Rng>(d: u64, rng: &mut R) -> SymBVPoly<F> {
        let mut coeffs: BTreeMap<(u64, u64), F> = Default::default();
        for deg_x in 0..=d/2 {
            for deg_y in 0..=d {
                let value =F::rand(rng); 
                coeffs.insert((deg_x, deg_y), value);
                if deg_x != deg_y {
                    coeffs.insert((deg_y, deg_x), value);
                }
            }
        }
        SymBVPoly {
            coeffs: coeffs,
            degree: d,
        }
    }

    pub fn eval(&self, x: F, y: F) -> F {
        let mut result = F::ZERO;
        for deg_x in (0..=self.degree).rev() {
            let mut result_intermediate = self.coeffs[&(self.degree, deg_x)]; //Note that deg_x always <= self.degree, hence the switch
            for deg_y in (1..=self.degree).rev() {
                if (deg_x >= (deg_y-1)) {
                    result_intermediate = self.coeffs[&(deg_x,deg_y-1)] + y*result_intermediate;
                } else {
                    result_intermediate = self.coeffs[&(deg_y-1,deg_x)] + y*result_intermediate;
                }
            }
            result = result_intermediate + x*result;
        }
        result
    }

    pub fn eval_std(&self, x: F, y: F) -> F {
        let mut result = F::ZERO;
        for deg_x in (0..=self.degree).rev() {
            let mut result_intermediate = F::ZERO;
            for deg_y in (0..=self.degree).rev() {
                if (deg_x >= deg_y) {
                    result_intermediate += self.coeffs[&(deg_x, deg_y)] * y.pow([deg_y]);
                } else {
                    result_intermediate += self.coeffs[&(deg_y, deg_x)] * y.pow([deg_y]);
                }
            }
            result = result_intermediate + x*result;
        }
        result
    }

    pub fn eval_new(&self, x: F, y: F) -> F {
        let mut result = F::ZERO;
        for deg_x in (0..=self.degree).rev() {
            for deg_y in (0..=self.degree).rev() {
                result = (result + self.coeffs[&(deg_x, deg_y)])*x + y*self.coeffs[&(deg_x, deg_y)];
            }
        }
        result
    }
}

impl<F: Field> Poly<F> {
    pub fn eval_std(&self, x: F) -> F {
        let mut result = F::ZERO;
        for deg_x in 0..=self.degree {
            result += self.coeffs[deg_x as usize] * x.pow([deg_x]);
        }
        result
    }

    pub fn eval(&self, x: F) -> F {
        let mut result = self.coeffs[self.degree as usize];
        for deg_x in (1..=self.degree).rev() {
            result = self.coeffs[(deg_x-1) as usize] + x*result;
        }
        result
    }

    pub fn evals_to_coeffs(x: &Vec<u64>, y: &Vec<F>, n: u64) -> Poly<F> {
        let mut full_coeffs: Vec<F> = vec![F::ZERO; n as usize];
        let mut terms: Vec<F> = vec![F::ZERO; n as usize];

        let mut prod: F;
        let mut degree = 0;
        for i in 0..=n-1 {
            prod = F::ONE; 

            for _j in 0..=n-1 {
                terms[_j as usize] = F::ZERO;
            }

            for j in 0..=n-1 {
                if i == j {
                    continue;
                } 
                prod *= F::from(x[i as usize]) - F::from(x[j as usize]);
            }

            prod = y[i as usize] / prod;

            terms[0] = prod;

            for j in 0..=n-1 {
                if i == j {
                    continue;
                }
                for k in (1..n).rev() {
                    let tmp_term = terms[(k - 1) as usize];
                    //dbg!(k, tmp_term);
                    terms[k as usize] += tmp_term;
                    terms[(k - 1) as usize] *= -F::from(x[j as usize]);
                }
            }

            for j in 0..=n-1 {
                full_coeffs[j as usize] += terms[j as usize];
            }
        }

        //for j in 0..=n-1 {
        //    dbg!(j, full_coeffs[j as usize]);
        //}
        for j in (0..=n-1).rev() {
            if full_coeffs[j as usize] != F::ZERO {
                //dbg!(j);
                degree = j;
                break;
            }
        }

        Poly {
            degree: degree,
            coeffs: full_coeffs
        }

    }
}



fn term_powers_for_degree(d: u64) -> impl Iterator<Item = (u64, u64)> {
    (0..=d)
    .flat_map(move |deg_x| (0..=d).map(move |deg_y| (deg_x, deg_y)))
}