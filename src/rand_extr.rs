use std::collections::BTreeMap;
use rand::Rng;

use std::time::SystemTime;
use std::mem::size_of_val;

pub struct Verifier {
    pub t: usize,
    pub index: usize,
    pub all_subsets: Vec<Vec<usize>>,
    pub my_leader_subsets: BTreeMap<usize, Vec<usize>>,
    pub my_participation_subsets: BTreeMap<usize, Vec<usize>>,
    pub values_of_participation_subsets: BTreeMap<usize, i32>,
    pub agreeable_parties_for_participation_subsets: BTreeMap<usize, Vec<usize>>
}

pub struct Publisher {
    pub t: usize,
    pub index: usize,
    pub all_subsets: Vec<Vec<usize>>,
    pub my_subsets: BTreeMap<usize, Vec<usize>>,
    pub received_values_of_participation_subsets: BTreeMap<usize, BTreeMap<usize, i32>>
}

pub struct Client {
    pub received_values_of_participation_subsets: BTreeMap<usize, BTreeMap<usize, i32>>
}

pub struct RandomnessExtractor {
    pub t: usize,
}

impl RandomnessExtractor {
    pub fn execute(&self) {
        let t = self.t;
        let mut comm_overall = 0.0;
        let start_time = SystemTime::now();
        //mesages to send to verifier: first to which verifier/publisher, then from which leader/verifier, then which subset and finally the random value
        let mut messages_to_send_to_verifiers_from_leaders: BTreeMap<usize, BTreeMap<usize, BTreeMap<usize, i32>>> = Default::default();
        let mut messages_to_send_to_verifiers_from_verifiers: BTreeMap<usize, BTreeMap<usize, BTreeMap<usize, i32>>> = Default::default();
        let mut messages_to_send_to_publishers: BTreeMap<usize, BTreeMap<usize, BTreeMap<usize, i32>>> = Default::default();
        let mut messages_send_by_publishers: BTreeMap<usize, BTreeMap<usize, i32>> = Default::default();

        for i in 1..=3*t + 1 {
            messages_to_send_to_verifiers_from_leaders.insert(i, BTreeMap::new());
            messages_to_send_to_verifiers_from_verifiers.insert(i, BTreeMap::new());
            messages_to_send_to_publishers.insert(i, BTreeMap::new());
            messages_send_by_publishers.insert(i, BTreeMap::new());

        }

        for i in 1..=3*t +1 {
            let mut verifier = Verifier {t: t, 
                                                index: dbg!(i), 
                                                all_subsets: Vec::new(), 
                                                my_leader_subsets: BTreeMap::new(), 
                                                my_participation_subsets: BTreeMap::new(), 
                                                values_of_participation_subsets : BTreeMap::new(), 
                                                agreeable_parties_for_participation_subsets: BTreeMap::new()};
            verifier.init();

            //If party is one of the leaders, we compute messages it sends to other verifiers as the leader
            if i < t + 2 {
                let messages_to_send_to_verifiers_as_leader = verifier.lead();
                comm_overall += (4.0*(messages_to_send_to_verifiers_as_leader.len() as f64) *
                (messages_to_send_to_verifiers_as_leader.values().last().unwrap().len() as f64))/1000000.0;

                for future_verifier in i..=3*t+1 {
                    let messages_to_this_verifier_from_this_leader = messages_to_send_to_verifiers_as_leader.get(&future_verifier).unwrap();
                    messages_to_send_to_verifiers_from_leaders.get_mut(&future_verifier)
                                                            .unwrap()
                                                            .insert(i, messages_to_this_verifier_from_this_leader.clone());
                }
            }

            let mut leader_messages_to_forward_to_verifiers_from_verifier: BTreeMap<usize, BTreeMap<usize, i32>> = Default::default();
            //If party received messages from leaders, we let it forward dealer's message to other verifiers 
            leader_messages_to_forward_to_verifiers_from_verifier = verifier.receive_from_leaders(messages_to_send_to_verifiers_from_leaders.get(&i).unwrap());
            comm_overall += (4.0*(leader_messages_to_forward_to_verifiers_from_verifier.len() as f64) 
            *(leader_messages_to_forward_to_verifiers_from_verifier.values().last().unwrap().len()as f64))/1000000.0;

            for future_verifier in i..=3*t+1 {
                let messages_to_this_verifier_as_participant = leader_messages_to_forward_to_verifiers_from_verifier.get(&future_verifier).unwrap();
                messages_to_send_to_verifiers_from_verifiers.get_mut(&future_verifier)
                                                            .unwrap()
                                                            .insert(i, messages_to_this_verifier_as_participant.clone());
            }

            //Verification phase: receive messages from prior verifiers
            verifier.receive_from_parties(messages_to_send_to_verifiers_from_verifiers.get(&i).unwrap());
            //Verification phase: finalize processing of all messages received
            let messages_to_send_to_publishers_as_participant = verifier.process_all_participation_subsets();
            comm_overall += (4.0*(messages_to_send_to_publishers_as_participant.len() as f64) *
            (messages_to_send_to_publishers_as_participant.values().last().unwrap().len()as f64))/1000000.0;

            for publisher in 1..=3*t+1 {
                let messages_to_this_publisher_from_this_verifier = messages_to_send_to_publishers_as_participant.get(&publisher).unwrap();
                messages_to_send_to_publishers.get_mut(&publisher)
                                                .unwrap()
                                                .insert(i, messages_to_this_publisher_from_this_verifier.clone());
            }
        }

        for i in 1..=3*t+1 {
            let mut publisher = Publisher {t: t, 
                                                    index: i, 
                                                    all_subsets: Vec::new(), 
                                                    my_subsets: BTreeMap::new(), 
                                                    received_values_of_participation_subsets : BTreeMap::new() };
            publisher.init();
            let messages_to_this_publisher = messages_to_send_to_publishers.get(&i).unwrap();
            let publisher_messages = publisher.process(messages_to_this_publisher);
            comm_overall += (4.0*(messages_send_by_publishers.values().last().unwrap().len() as f64))/1000000.0;

            messages_send_by_publishers.insert(i, publisher_messages);
        }

        let client = Client{received_values_of_participation_subsets: messages_send_by_publishers };

        let end_time = SystemTime::now();
        let duration = end_time.duration_since(start_time).unwrap();
        println!("Whole protocol takes {} milliseconds", duration.as_millis());
        println!("Whole protocol has comm {}", comm_overall);

    }
}

impl Publisher {
    pub fn init(&mut self) {
        let start_time = SystemTime::now();
        let mut all_subsets: Vec<Vec<usize>> = Vec::new();
        let mut current_subset: Vec<usize> = Vec::new();
        //let nums = (1..=3 * t + 1).collect::<Vec<_>>();
        let max_num = 3*self.t + 1;
        let subset_size = 2*self.t as usize + 1;

        generate_subsets(max_num, subset_size, 1, &mut current_subset, &mut all_subsets);
        let mut my_subsets: BTreeMap<usize,Vec<usize>> = Default::default();

        let number_subsets = all_subsets.len();

        for subset_index in 0..number_subsets {
            if (all_subsets[subset_index].contains(&self.index)) {
                my_subsets.insert(subset_index, all_subsets[subset_index].clone());
            }
        }

        self.all_subsets = all_subsets;
        self.my_subsets = my_subsets;


        let end_time = SystemTime::now();
        let duration = end_time.duration_since(start_time).unwrap();
        //println!("Publisher init takes {} milliseconds", duration.as_millis());

    }

    pub fn process(&mut self, random_values_from_verifiers: &BTreeMap<usize, BTreeMap<usize, i32>>) -> BTreeMap<usize, i32>{

        let start_time = SystemTime::now();
        let mut subset_results: BTreeMap<usize, i32> = BTreeMap::new();

        for subset_index in self.my_subsets.keys() {
            let mut number_votes_for_zero = 0;
            let mut number_votes_for_one = 0;
            for verifier in self.my_subsets.get(subset_index).unwrap() {
                if random_values_from_verifiers.get(verifier).unwrap().get(subset_index) == Some(&1) {
                    number_votes_for_one += 1;
                } else {
                    number_votes_for_zero += 0;
                }
            }
            if number_votes_for_one > number_votes_for_zero {
                subset_results.insert(*subset_index, 1);
                //publish this output
                //println!("One won for {}", *subset_index)
            }  else {
                subset_results.insert(*subset_index, 0);
                //println!("Zero won for {}", *subset_index)
            }
        }

        let end_time = SystemTime::now();
        let duration = end_time.duration_since(start_time).unwrap();

        subset_results
        //println!("Publsiher processing takes {} milliseconds", duration.as_millis());
    }
}

impl Client {
    pub fn compute_coin(&mut self) {
        let coin = 0;

        let mut count_maj_for_subsets: BTreeMap<usize, i32> = BTreeMap::new();

        for publisher in self.received_values_of_participation_subsets.keys() {
            for subset in self.received_values_of_participation_subsets.get(publisher).unwrap().keys() {
                if self.received_values_of_participation_subsets.get(publisher).unwrap().get(subset) == Some(&1) {
                    count_maj_for_subsets.insert(*subset, count_maj_for_subsets[subset] + 1);
                } else {
                    count_maj_for_subsets.insert(*subset, count_maj_for_subsets[subset] - 1);
                }
            }
        }

        for subset in count_maj_for_subsets.keys() {
            if count_maj_for_subsets[subset] > 0 {
                //Ones won
                coin != coin; 
            }
        }
    }

}

impl Verifier {
    pub fn init(&mut self) {

        let start_time = SystemTime::now();

        let mut all_subsets: Vec<Vec<usize>> = Vec::new();
        let mut current_subset: Vec<usize> = Vec::new();
        //let nums = (1..=3 * t + 1).collect::<Vec<_>>();
        let max_num = 3*self.t + 1;
        let subset_size = 2*self.t as usize + 1;

        generate_subsets(max_num, subset_size, 1, &mut current_subset, &mut all_subsets);

        let mut my_leader_subsets: BTreeMap<usize,Vec<usize>> = Default::default();
        let number_subsets = all_subsets.len();

        for subset_index in 0..number_subsets {
            if all_subsets[subset_index][0] == self.index {
                my_leader_subsets.insert(subset_index, all_subsets[subset_index].clone());
            }
        }
        let mut my_participation_subsets: BTreeMap<usize,Vec<usize>> = Default::default();

        for subset_index in 0..number_subsets {
            if all_subsets[subset_index].contains(&self.index) {
                my_participation_subsets.insert(subset_index, all_subsets[subset_index].clone());
            }
        }

        self.all_subsets = all_subsets;
        self.my_leader_subsets = my_leader_subsets;
        self.my_participation_subsets = my_participation_subsets;

        let end_time = SystemTime::now();
        let duration = end_time.duration_since(start_time).unwrap();
        //println!("Verifier init takes {} milliseconds", duration.as_millis());
    }


    //Return a map of <verifier_to_send_msg_to, <subset_index, random_value>>
    pub fn lead(&self) -> BTreeMap<usize, BTreeMap<usize, i32>> {

        let start_time = SystemTime::now();

        let mut rng = rand::thread_rng();
        let mut messages_to_send_to_verifiers: BTreeMap<usize, BTreeMap<usize, i32>> = Default::default();

        for verifier in 1..=3*self.t + 1 {
            messages_to_send_to_verifiers.insert(verifier, BTreeMap::new());
        }

        for (subset_index,subset) in &self.my_leader_subsets {
            let random_value = rng.gen_bool(0.5) as i32;

            for verifier in subset {
                let messages_to_sent_to_verifier = messages_to_send_to_verifiers.get_mut(&verifier).unwrap();
                messages_to_sent_to_verifier.insert(*subset_index, random_value);
            }
        }

        let end_time = SystemTime::now();
        let duration = end_time.duration_since(start_time).unwrap();
        //println!("Verifier lead takes {} milliseconds", duration.as_millis());

        messages_to_send_to_verifiers

    }
    pub fn receive_from_leaders(&mut self, random_values_from_dealers: &BTreeMap<usize,BTreeMap<usize,i32>>) -> BTreeMap<usize, BTreeMap<usize, i32>> {
        let start_time = SystemTime::now();

        let mut messages_to_send_to_verifiers: BTreeMap<usize, BTreeMap<usize, i32>> = Default::default();
        for verifier in self.index..=3*self.t + 1 {
            messages_to_send_to_verifiers.insert(verifier, BTreeMap::new());
        }

        //We will go through all dealers that were executed before this party
        let mut max_dealer = self.t + 1;
        if (self.index < max_dealer) {
            max_dealer = self.index;
        }

        for current_leader in 1..=max_dealer {
            let current_dealer_subsets = random_values_from_dealers.get(&current_leader).unwrap().keys();
            
            for subset_index in  current_dealer_subsets{
                let random_value = random_values_from_dealers[&current_leader][&subset_index];
                //Store x^j_S as the set value received by the corresponding dealer
                self.values_of_participation_subsets.insert(*subset_index, random_value);

                //Send x^j_S to all verifiers in the corresponding subset down the line
                for verifier in self.my_participation_subsets.get(subset_index).unwrap() {
                    if *verifier > self.index - 1 {
                        let messages_to_send_to_verifier = messages_to_send_to_verifiers.get_mut(&verifier).unwrap();
                        messages_to_send_to_verifier.insert(*subset_index, random_value);
                    } 
                }
            }
        }
        let end_time = SystemTime::now();
        let duration = end_time.duration_since(start_time).unwrap();
        //println!("Verifier receive from leaders takes {} milliseconds", duration.as_millis());
        messages_to_send_to_verifiers
    }

    pub fn receive_from_parties(&mut self, random_values_from_prior_parties: &BTreeMap<usize, BTreeMap<usize,i32>>) {
        let start_time = SystemTime::now();

        for (subset_index, subset) in &self.my_participation_subsets {
            self.agreeable_parties_for_participation_subsets.insert(*subset_index, Default::default());
        }

        //Go through each prior party and messages we received from that party
        for current_party in 1..=self.index {
            for (subset_index, random_value) in random_values_from_prior_parties.get(&current_party).unwrap() {
                //If the subset value we received from the party is not consistent with what the leader of that subset sent us, we coomplain
                if self.values_of_participation_subsets.get(&subset_index) != Some(random_value) {
                    //complain here
                } else {
                    //If the values are consistent, we add the party to the set of party which agree for that particular subset
                    let agreable_parties_for_subset =  self.agreeable_parties_for_participation_subsets.get_mut(subset_index).unwrap();
                    agreable_parties_for_subset.push(current_party);
                }
            }
        }

        let end_time = SystemTime::now();
        let duration = end_time.duration_since(start_time).unwrap();
        //println!("Verification msg processing takes {} milliseconds", duration.as_millis());
    }

    pub fn process_all_participation_subsets(&mut self) -> BTreeMap<usize, BTreeMap<usize, i32>> {
        let start_time = SystemTime::now();

        let mut messages_to_send_to_publishers: BTreeMap<usize, BTreeMap<usize, i32>> = Default::default();
        for publisher in 1..=3*self.t + 1 {
            let subset_map: BTreeMap<usize, i32> = BTreeMap::new();
            messages_to_send_to_publishers.insert(publisher, subset_map);
        }
        //Go through all subsets in which I participated
        for (subset_index, subset) in &self.my_participation_subsets {
            let mut subset_not_complete = false;
            //Check if all parties which were supposed to agree actually agreed
            for party in subset {
                if (*party < self.index) && !self.agreeable_parties_for_participation_subsets
                                                                .get(&subset_index)
                                                                .unwrap()
                                                                .contains(&party) {
                    subset_not_complete = true;
                    break;
                }
            }
            //If we received all the values that we anticipated, we proceed by including these messages into the list that we will send to publishers
            if !subset_not_complete {
                for publisher in subset {
                    messages_to_send_to_publishers.get_mut(&publisher).unwrap().insert(*subset_index, 
                        *self.values_of_participation_subsets.get(&subset_index).unwrap());                        
                }
            }
        }

        let end_time = SystemTime::now();
        let duration = end_time.duration_since(start_time).unwrap();
        //println!("Verification msg generation takes {} milliseconds", duration.as_millis());
        messages_to_send_to_publishers
    }
}

pub fn generate_subsets(max_num: usize, subset_size: usize, start_idx: usize, current_subset: &mut Vec<usize>, all_subsets: &mut Vec<Vec<usize>>) {
    if current_subset.len() == subset_size {
        all_subsets.push(current_subset.clone());
        return;
    }

    for i in start_idx..=max_num {
        current_subset.push(i);
        generate_subsets(max_num, subset_size, i + 1, current_subset, all_subsets);
        current_subset.pop();
    }
}
