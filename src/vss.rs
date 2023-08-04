use std::{collections::BTreeMap};
use ark_ff::BigInteger;
use ark_ff::{Field};
use ark_std::test_rng;

//use pqcrypto_sphincsplus::sphincssha256128fsimple::*;
use crate::polynomials::Poly;
use crate::polynomials::SymBVPoly;
use crate::communication::*;
use std::mem::size_of_val;
use std::time::{SystemTime, Duration};
use ed25519_dalek::*;
use rand::rngs::OsRng;

use ark_std::rand::prelude::StdRng;

use ark_std::rand::SeedableRng;
pub struct PubParams {
    //Number of potentially adversarial parties
    pub t: u64,
    //Number of receivers
    pub n: u64,
    //Total umber of parties
    pub n_parties_total: u64,
    //pub sig_pp: Parameters<Edwards, Blake2s>
}

pub struct VSS<F: Field> {
    pub secret: F,
    pub pp: PubParams,
    pub execution_leaks: bool
}

pub struct RandExtractorVSSBased<F: Field> {
    pub secret: F,
    pub pp: PubParams,
    pub execution_leaks: bool
}

pub struct Dealer<'a, F: Field> {
    pub pp: &'a PubParams,
    pub secret: F,
    //pk: Option<PublicKey>,
    //sk: Option<SecretKey>
    keypair: Option<Keypair>
}

pub struct Resolver<'a, F: Field> {
    pub pp: &'a PubParams,
    pub secret: F,
    pub bc: &'a BroadcastChannel,
    pub pcs: &'a Vec<Vec<PrivateChannel>> 
}

pub struct Receiver<'a, F: Field> {
    pub id: u64,
    pub pp: &'a PubParams,
    //pk: Option<PublicKey>,
    //sk: Option<SecretKey>
    keypair: Option<Keypair>,
    share: BTreeMap<u64, Subshare<F>>
}

//impl<'a, F: Field> Receiver<'a, F> {
//    fn modify_share(&mut self, share: &BTreeMap<u64, Subshare<F>>) {
//        self.share = share.clone();
//    }
//}

pub struct Reconstructor<'a, F: Field> {
    pub pp: &'a PubParams,
    _marker: std::marker::PhantomData<F>,
}

pub struct Client<'a, F: Field> {
    pub pp: &'a PubParams,
    _marker: std::marker::PhantomData<F>
}

///Subshare consists of value, and a subset of three signarures on this value
#[derive(Clone)]
pub struct Subshare<F: Field> {
    value: F,
    dealer_signature: Option<Signature>,
    p_i_signature: Option<Signature>,
    p_j_signature: Option<Signature>
}

impl<F: Field> Subshare<F> {
    fn modify_dealer_signature(&mut self, signature: &Signature) {
        self.dealer_signature = Some(signature.clone());
    }
    fn modify_p_i_signature(&mut self, signature: &Signature) {
        self.p_i_signature = Some(signature.clone());
    }
    fn modify_p_j_signature(&mut self, signature: &Signature) {
        self.p_j_signature = Some(signature.clone());
    }
}



impl<F: Field> VSS<F> {
    pub fn execute(&self) {
        let t = self.pp.t;
        let n = self.pp.n;

        let mut dealer_time = Duration::new(0,0);
        let mut receiver_time: Vec<Duration> = Vec::new();
        let mut reconstructor_time = Duration::new(0,0);
        let mut client_time = Duration::new(0,0);


        let mut dealer_comm =0.0;
        let mut receiver_comm: Vec<f64> = Vec::new();
        let mut reconstructor_comm = 0.0;
        let mut client_comm = 0.0;

        //let mut pcs: Vec<Vec<PrivateChannel>> = vec![Vec::new(); n_parties_total as usize];
        let mut pki: Vec<PublicKey> = Vec::new();
        let mut dealer: Dealer<F> = Dealer{secret: self.secret, pp: &self.pp, keypair: None};

        let dealer_start_time = SystemTime::now();        
        //Dealer shares the secret, gather secret shares
        let (shares, pk) = dealer.share();
        let dealer_end_time = SystemTime::now();
        dealer_time = dealer_end_time.duration_since(dealer_start_time).unwrap();
        println!("Dealer's work takes {} milliseconds", dealer_time.as_millis());

        dealer_comm = (((size_of_val(&shares[&1][&1]) as u64)*(shares.len() as u64) *(shares.len() as u64) + (size_of_val(&pk) as u64)) as f64)/1000000.0;

        pki.push(pk);
        //Stoore shares th
        let mut shares_signed_by_p_i: BTreeMap<u64, BTreeMap<u64, Subshare<F>>> = Default::default();
        let mut shares_triple_signed: BTreeMap<u64, BTreeMap<u64, Subshare<F>>> = Default::default();

        //Each receiver verifies what it got from the dealer and what it got from other parties, compute what it wants to send to other parties
        for i in 1..=n {
            //need to forward these triply shares to the reconstructors
            let mut expanded_shares: BTreeMap<u64, Subshare<F>> = Default::default();
            let mut receiver_i: Receiver<F> = Receiver{ id: i, pp: &self.pp, keypair: None, share: BTreeMap::new() };
            //need to forward these doubly shares to future receivers

            let receiver_start_time = SystemTime::now();
            let (receiver_i_shares_to_send, pk_p_i) =
                                        receiver_i.receive_from_dealer(&pk, &shares[&i]);
            pki.push(pk_p_i);
            shares_signed_by_p_i.insert(i, receiver_i_shares_to_send);
            for j in 1..=i {
                let (happy, expanded_share) = receiver_i.receive_from_party(j, 
                                                        &shares_signed_by_p_i[&j][&i],
                                                        &pki[0],
                                                        &pki[j as usize]);
                if happy {
                    expanded_shares.insert(j, expanded_share);
                }
            }

            let receiver_end_time = SystemTime::now();
            receiver_time.push(receiver_end_time.duration_since(receiver_start_time).unwrap());

            receiver_comm.push((((size_of_val(&expanded_shares[&1]) as u64)*(expanded_shares.len() as u64)*(t+1) +
            (size_of_val(&shares_signed_by_p_i[&1][&1]) as u64)*(shares_signed_by_p_i[&1].len() as u64) + 
            (size_of_val(&pk_p_i) as u64)) as f64)/1000000.0);
            
            shares_triple_signed.insert(i, expanded_shares);
        }

        //Reconstructors publish projections that they received
        for _i in 1..=t+1 {
            let reconstructor: Reconstructor<F> = Reconstructor{pp: &self.pp, _marker: std::marker::PhantomData};
            for _j in 1..=n {
                reconstructor.receive_from_party(_j, &shares_triple_signed[&_j], &pki);
            }
        }
        reconstructor_comm = (((size_of_val(&shares_triple_signed[&1][&1]) as f64)*dbg!(shares_triple_signed.len()) as f64 )*dbg!(shares_triple_signed[&1].len())
    as f64)/1000000.0;

        let client: Client<F> = Client { pp: &self.pp, _marker: std::marker::PhantomData };

        let client_start_time = SystemTime::now();
        let secret: (bool, F) = client.compute_secret(&shares_triple_signed, &pki);
        let client_end_time = SystemTime::now();
        client_time = client_end_time.duration_since(client_start_time).unwrap();
        println!("Dealer's work takes {} milliseconds", dealer_time.as_millis());
        println!("First receiver's work takes {} milliseconds", receiver_time[0].as_millis());
        println!("Last receiver's work takes {} milliseconds", receiver_time[(n - 1) as usize].as_millis());
        println!("Client's work takes {} milliseconds", client_time.as_millis());


        println!("Dealer requires {} MB", dealer_comm);
        println!("First receiver requires {} MB", receiver_comm[0]);
        println!("Last receiver requires {} MB", receiver_comm[receiver_comm.len() - 1]);
        println!("Reconstructor requires {} MB", reconstructor_comm);


        let mut time_per_party: Vec<Duration> = Vec::new();
        let mut comm_per_party: Vec<f64> = Vec::new();

        let mut overall_time = 0;
        let mut overall_comm = 0.0;

        for i in 1..=5*t + 4 {
            time_per_party.push(Duration::new(0,0));
            comm_per_party.push(0.0);
        }

        //first t+1 parties are acting as dealers
        for i in 1..=t + 1 {
            time_per_party[(i - 1) as usize] += dealer_time;
            comm_per_party[(i - 1) as usize] += dealer_comm;

            for k in (i + 1)..=3*t + i + 1 {
                time_per_party[(k - 1) as usize] += receiver_time[(k - 1 - i) as usize];
                comm_per_party[(k - 1) as usize] += receiver_comm[(k - 1 - i) as usize];
            }
        }

        for k in 4*t + 4..=5*t + 4 {
            comm_per_party[(k - 1) as usize] += reconstructor_comm;
        }

        for i in 1..=5*t + 4 {
            overall_time += time_per_party[(i-1) as usize].as_millis();
            overall_comm += comm_per_party[(i-1) as usize];
        }

        overall_time += client_time.as_millis();

        println!("Overall time: {}", overall_time);
        println!("Overall comm: {}", overall_comm);


        dbg!(secret);

    }
}

impl<F: Field> Dealer<'_, F> {
    fn set_key_pair(&mut self) {
        // Generating signature key pair
        //let (pk, sk) = keypair();
        //self.sk = Some(sk);
        //self.pk = Some(pk);

        //let sig_pp = SchnorrEdwards::setup(&mut test_rng()).unwrap();
        //let (pk,sk) = SchnorrEdwards::keygen(&sig_pp, &mut test_rng()).unwrap();
        let mut csprng = OsRng{};
        let keypair: Keypair = Keypair::generate(&mut csprng);
        self.keypair = Some(keypair);
    }

    pub fn share(&mut self) -> (BTreeMap<u64, BTreeMap<u64, Subshare<F>>>, PublicKey) {
        let n = self.pp.n;

        let mut csprng: StdRng = StdRng::seed_from_u64(6);
        // Generating random symmetric bivariate polynomial to hold the secret
        let poly = SymBVPoly::rand(self.pp.t, &mut csprng);

        // Generate and set a signature key pair
        self.set_key_pair();

        let keypair = self.keypair.as_ref().unwrap();

        // Generate signed shares for all parties
        let shares: BTreeMap<u64, BTreeMap<u64, Subshare<F>>> = (1..=n)
            .map(|i| {
                (
                    i,
                    (1..=n)
                        .map(|j| {
                            //Compute the share
                            let share = poly.eval(F::from(i), F::from(j));

                            //Serialize and generate dealer's signature
                            let mut share_bytes = Vec::new();
                            //Try (share, F::from(i), F::from(j)) here instead
                            share.serialize_uncompressed(&mut share_bytes).unwrap();

                            let dealer_signature = keypair.sign(&mut share_bytes); 

                            (j, Subshare { value: share, dealer_signature: Some(dealer_signature), p_i_signature: None, p_j_signature: None })
                        })
                        .collect(),
                )
            })
            .collect();

        (shares,  keypair.public)
    }
}


impl<F: Field> Receiver<'_, F> {
    fn set_key_pair(&mut self) {
        // Generating signature key pair
        let mut csprng = OsRng{};
        let keypair: Keypair = Keypair::generate(&mut csprng);
        self.keypair = Some(keypair);
    }
    
    pub fn receive_from_dealer(&mut self, dealer_pk: &PublicKey, share: &BTreeMap<u64, Subshare<F>>) 
                                                    -> (BTreeMap<u64, Subshare<F>>, PublicKey) {
        let n = self.pp.n;
        let t = self.pp.t;      
        self.set_key_pair();
        let keypair = self.keypair.as_ref().unwrap();

        let x_vals: Vec<u64> = share.keys().cloned().collect();
        let yvals_signed: Vec<Subshare<F>> = share.values().cloned().collect();

        //Verify whether each subshare is correctly signed by the dealer
        for subshare in yvals_signed {
            let message = subshare.value;
            //let dealer_msg = open(&subshare.dealer_signature.unwrap(), &dealer_pk).unwrap();
            //let uncompressed_dealer_msg: F = F::deserialize_uncompressed(&*dealer_msg).unwrap();
            let mut share_bytes = Vec::new();
            message.serialize_uncompressed(&mut share_bytes).unwrap();
            if !dealer_pk.verify(&share_bytes, &subshare.dealer_signature.unwrap()).is_ok() {
                println!("I'm unhappy!")
            } 
        }

        //let yvals_signed: Vec<Subshare<F>> = share.values().cloned().collect();
        let y_vals: Vec<F> = share.values().into_iter().map(|signed_share| signed_share.value).collect();

        let unipoly: Poly<F> = Poly::evals_to_coeffs(&x_vals, &y_vals, n);

        if unipoly.degree > t {
            println!("I'm unhappy!")
        }

        //I'm happy, preparing doubly signed subshares 
        let subshares_doubly_signed: BTreeMap<u64, Subshare<F>> = (self.id..=n)
                                    .map(|k| {
                                        let mut share_bytes = Vec::new();
                                        share[&k].value.serialize_uncompressed(&mut share_bytes).unwrap();
                                        let p_i_signature = keypair.sign(&mut share_bytes); 
                                        let dealer_signature = share[&k].dealer_signature.clone();
                                        (k, Subshare{ value: share[&k].value, dealer_signature: dealer_signature, p_i_signature: Some(p_i_signature), p_j_signature: None })
                                    } )
                                    .collect();
        self.share = share.clone();
        (subshares_doubly_signed, keypair.public)
    }

    pub fn receive_from_party(&self, from: u64, share: &Subshare<F>, dealer_pk: &PublicKey, pk_i: &PublicKey) -> (bool,Subshare<F>) {
        let mut happy = true; //I'm happy
        let keypair = self.keypair.as_ref().unwrap();
        //verify dealer's signature first
        let message = share.value;
        let mut share_bytes = Vec::new();
        message.serialize_uncompressed(&mut share_bytes).unwrap();
        //let dealer_msg = open(&share.dealer_signature.clone().unwrap(), dealer_pk).unwrap();
        //let dealer_msg_uncompressed: F = F::deserialize_uncompressed(&*dealer_msg).unwrap();
        
        //let p_i_msg = open(&share.p_i_signature.clone().unwrap(), pki).unwrap();
        //let p_i_uncompressed: F = F::deserialize_uncompressed(&*p_i_msg).unwrap();

        if !dealer_pk.verify(&share_bytes, &share.dealer_signature.unwrap()).is_ok() ||
            !pk_i.verify(&share_bytes, &share.p_i_signature.unwrap()).is_ok() {
            happy = false; //Unhappy because one of the signatures does not verify :(
            println!("I'm unhappy!")
        } else { 
            if self.share[&from].value != share.value {
                happy = false; //Unhappy because party claims different message :(
                println!("Must complain!"); //dealer malicious
            } 
            //dbg!(self.i, from);
        }
        
        let dealer_signature = share.dealer_signature.clone();
        let p_i_signature = share.p_i_signature.clone();        
        //let mut message_bytes = Vec::new();
        //share.value.serialize_uncompressed(&mut message_bytes).unwrap();
        let p_j_signature = keypair.sign(&mut share_bytes);
        let expanded_share: Subshare<F> = Subshare {value: share.value, dealer_signature: dealer_signature, p_i_signature: p_i_signature, p_j_signature: Some(p_j_signature)};

        return (happy, expanded_share)
    }
}

impl<F: Field> Reconstructor<'_, F> {
    pub fn receive_from_party(&self, from: u64, triply_signed_shares: &BTreeMap<u64, Subshare<F>>, pki: &Vec<PublicKey>) 
                                        -> BTreeMap<u64, Subshare<F>> {

        // let xvals: Vec<u64> = triply_signed_shares.keys().cloned().collect();
        // //let yvals_tmp: Vec<Subshare<F>> = triply_signed_shares.values().cloned().collect();
        // let yvals: Vec<F> = triply_signed_shares.values().map(|signed_share| signed_share.value).collect();

        // let mut verified_shares: BTreeMap<u64, Subshare<F>> = Default::default();
        // //let yvals_signed: Vec<Subshare<F>> = triply_signed_shares.values().cloned().collect();


        // for key in xvals {
        //     let share = &triply_signed_shares[&key];
        //     //Verify share;
        //     let message = share.value;
        //     let dealer_msg = open(&share.dealer_signature.clone().unwrap(), &pki[0]).unwrap();
        //     let dm_uncompressed: F = F::deserialize_uncompressed(&*dealer_msg).unwrap();
        
        //     let pi_msg = open(&share.p_i_signature.clone().unwrap(), &pki[key as usize]).unwrap();
        //     let pi_uncompressed: F = F::deserialize_uncompressed(&*pi_msg).unwrap();

        //     let pj_msg = open(&share.p_j_signature.clone().unwrap(), &pki[from as usize]).unwrap();
        //     let pj_uncompressed: F = F::deserialize_uncompressed(&*pj_msg).unwrap();

        //     if dm_uncompressed != message || pi_uncompressed != message || pj_uncompressed != message {
        //         println!("I'm unhappy!");
        //         continue;
        //     }
        //     let share = Subshare { value: share.clone().value, 
        //                                         dealer_signature: share.dealer_signature.clone(), 
        //                                         p_i_signature: share.p_i_signature.clone(),
        //                                         p_j_signature: share.p_j_signature.clone() };
        //     verified_shares.insert(key, share);
        // }
        
        return triply_signed_shares.clone()
    }
}


impl<F: Field> Client<'_, F> {
    pub fn compute_secret(&self, triply_signed_shares: &BTreeMap<u64, BTreeMap<u64, Subshare<F>>>, pki: &Vec<PublicKey>) -> (bool,F) {
        let mut secret_computable = true; 
        let n = self.pp.n;
        let t = self.pp.t;

        let mut verified_shares_of_zero_poly_keys: Vec<u64> = Default::default();
        let mut verified_shares_of_zero_poly_values: Vec<F> = Default::default();
        let mut n_verified_poly = 0;

        for i in 1..=n {

            //let xvals: Vec<u64> = triply_signed_shares[&i].keys().cloned().collect();
            //let yvals_tmp: Vec<Subshare<F>> = triply_signed_shares.values().cloned().collect();
            //triply_signed_shares[&i].values().map(|signed_share| signed_share.value).collect();

            let mut verified_share_keys: Vec<u64> = Default::default();
            let mut verified_share_values: Vec<F> = Default::default();
            //let yvals_signed: Vec<Subshare<F>> = triply_signed_shares.values().cloned().collect();

            let mut shares_verified = 0;

            for key in 1..=n {
                let mut smaller_index;
                let mut larger_index;
                if key <= i {
                    smaller_index = key;
                    larger_index = i;
                } else {
                    smaller_index = i;
                    larger_index = key;
                }
                let share: &Subshare<F> = &triply_signed_shares[&larger_index][&smaller_index];

                let message = share.value;
                let mut share_bytes = Vec::new();
                message.serialize_uncompressed(&mut share_bytes).unwrap();

                //let dealer_msg = open(&share.dealer_signature.clone().unwrap(), &pki[0]).unwrap();
                //let dm_uncompressed: F = F::deserialize_uncompressed(&*dealer_msg).unwrap();
            
                //let pi_msg = open(&share.p_i_signature.clone().unwrap(), &pki[smaller_index as usize]).unwrap();
                //let pi_uncompressed: F = F::deserialize_uncompressed(&*pi_msg).unwrap();

                //let pj_msg = open(&share.p_j_signature.clone().unwrap(), &pki[larger_index as usize]).unwrap();
                //let pj_uncompressed: F = F::deserialize_uncompressed(&*pj_msg).unwrap();

                if !pki[0].verify(&share_bytes, &share.dealer_signature.unwrap()).is_ok() ||
                        !pki[smaller_index as usize].verify(&share_bytes, &share.p_i_signature.unwrap()).is_ok() ||
                        !pki[larger_index as usize].verify(&share_bytes, &share.p_j_signature.unwrap()).is_ok()  {
                    println!("I'm unhappy!");
                    break;
                }
                verified_share_keys.push(key);
                verified_share_values.push(message);
                shares_verified +=1;
                if shares_verified >= 2*t  + 1 {
                    break;
                }
            }
            
            //Skip this party if we don't have enough verified subshares
            if shares_verified < 2*t  + 1 {
                continue;
            }

            let unipoly: Poly<F> = Poly::evals_to_coeffs(&verified_share_keys, &verified_share_values, shares_verified);

            if unipoly.degree > t {
                println!("I'm unhappy!")
            }
            else {
                n_verified_poly += 1;
                verified_shares_of_zero_poly_keys.push(i);
                verified_shares_of_zero_poly_values.push(unipoly.eval(F::ZERO));
            }
            if n_verified_poly >= t  + 1 {
                break;
            }
        }


        if n_verified_poly < t  + 1 {
            println!("I'm unhappy!")
        }

        let unipoly: Poly<F> = Poly::evals_to_coeffs(&verified_shares_of_zero_poly_keys, &verified_shares_of_zero_poly_values, n_verified_poly);

        let secret: F = unipoly.eval(F::ZERO);
        secret_computable = n_verified_poly > t;
        (secret_computable, secret)
    }
}





//let message = Subshare { value: F::ONE, dealer_signature: None, pi_signature: None, pj_signature: None };
//let mut message_bytes = Vec::new();
//F::ONE.serialize_uncompressed(&mut message_bytes).unwrap();
//let sm = sign(&message_bytes, &sk);
//let verifiedmsg = open(&sm, &pk).unwrap();
//let m_uncompressed: F = F::deserialize_uncompressed(&*verifiedmsg).unwrap();