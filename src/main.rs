#![allow(unused_imports)]
#![allow(warnings, unused)]

use ark_bls12_381::Fq;
mod vss;
mod polynomials;
mod communication;
mod rand_extr;
use vss::{VSS, PubParams};
use rand_extr::{RandomnessExtractor};

fn main() {
    let t: u64 = 7;

    let pp = PubParams {t: t, n: 3*t + 1, n_parties_total: 5*t+4};
    let execution_leaks = false;
    //let vss: VSS<Fq> = VSS { secret: 1.into(), pp: pp, execution_leaks: execution_leaks};
    //vss.execute();

    let rand_extr = RandomnessExtractor {t: t as usize}; 
    rand_extr.execute(); 

    
}
