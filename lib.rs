#![cfg_attr(not(feature = "std"), no_std)]

#[ink::contract]
mod erc20 {
    use ink::storage::Mapping;


    #[ink(storage)]
    pub struct Erc20 {
        total_supply: Balance,
        balances: Mapping<AccountId, Balance>,
        allowances: Mapping<(AccountId, AccountId), Balance>,
        is_voting_happening: bool,
        votes: Mapping<u128, u128>,
        time_to_vote: u64,
        voting_begin_time: u64,
        voting_end_time: u64,
        already_voted: Mapping<AccountId, bool>,
        current_winner: u128,
        voting_number: u32,
        token_price: u128,
        fee: u128,
        fee_divider: u128,
        weekly_fee_to_burn: u128,
        time_lapsed_for_fee_to_burn: u64,
    }

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        InsufficientBalance,
        InsufficientAllowance,
        VotingIsNotOngoing,
        VotingIsAlreadyOngoing,
        AccountAlreadyVoted,
        AccountFrozenBecauseVoted,
        TimeToVoteNotElapsed,
        ExactAmountOfWeiRequired,
        TimeForFeeBurnHasNotLapsed
    }

    pub type Result<T> = core::result::Result<T, Error>;

    #[ink(event)]
    pub struct Transfer {
        #[ink(topic)]
        from: Option<AccountId>,
        #[ink(topic)]
        to: AccountId, 
        value: Balance
    }

    #[ink(event)]
    pub struct Approval {
        #[ink(topic)]
        owner: AccountId,
        #[ink(topic)]
        spender: AccountId,
        value: Balance,
    }

    #[ink(event)]
    pub struct VotingStartedTime {
        #[ink(topic)]
        voting_begin_time: u64,
        voting_number: u32
    }

    impl Erc20 {
        #[ink(constructor)]
        pub fn new(total_supply: Balance) -> Self {
            let mut balances = Mapping::default();
            let caller = Self::env().caller();
            balances.insert(caller, &total_supply);
            Self::env().emit_event(Transfer {
                from: None,
                to: caller,
                value: total_supply,
            });
            Self { 
                total_supply, 
                balances,             
                allowances: Mapping::default(),
                is_voting_happening: false,
                votes: Mapping::default(),
                time_to_vote: 86400,
                voting_begin_time: Default::default(),
                voting_end_time: Default::default(),
                already_voted: Mapping::default(),
                current_winner: Default::default(),
                voting_number: Default::default(),
                token_price: 5,
                fee: 1,
                fee_divider: 100,
                weekly_fee_to_burn: Default::default(),
                time_lapsed_for_fee_to_burn: Default::default()
            }
        }

        #[ink(message)]
        pub fn total_supply(&self) -> Balance {
            self.total_supply
        }

        #[ink(message)]
        pub fn balance_of(&self, owner: AccountId) -> Balance {
            self.balances.get(owner).unwrap_or_default()
        }

        #[ink(message)]
        pub fn allowances(&self, token_owner: AccountId, spender: AccountId) -> Balance {
            self.allowances.get(&(token_owner, spender)).unwrap_or_default()
        }

        #[ink(message)]
        pub fn get_current_timestemp(&self) -> u64 {
            self.env().block_timestamp()
        }

        #[ink(message)]
        pub fn get_current_block_number(&self) -> u32 {
            self.env().block_number()
        }

        #[ink(message)]
        pub fn transfer(&mut self, to: AccountId, tokens: u128) -> Result<()>{
            let msg_sender: AccountId = self.env().caller();
            let msg_sender_balance: Balance = self.balance_of(msg_sender);
            if msg_sender_balance < tokens {
                return Err(Error::InsufficientBalance);
            } 
            if self.already_voted.get(msg_sender).unwrap_or_default() {
                return Err(Error::AccountFrozenBecauseVoted);
            }
            self.balances.insert(&msg_sender, &(msg_sender_balance - tokens));
            let to_balance = self.balance_of(to);
            self.balances.insert(&to, &(to_balance + tokens));
            //event
            self.env().emit_event(Transfer{
                from: Some(msg_sender), 
                to: to, 
                value: tokens
            });
            Ok(())
        }

        #[ink(message)]
        pub fn transfer_from(&mut self, from: AccountId, to: AccountId, tokens: Balance) -> Result<()> {
            let msg_sender_balance = self.balance_of(from);
            if msg_sender_balance < tokens {
                return Err(Error::InsufficientBalance);
            }
            let allowance = self.allowances(from, to);
            if allowance < tokens {
                return Err(Error::InsufficientAllowance)
            }
            if self.already_voted.get(from).unwrap_or_default() {
                return Err(Error::AccountFrozenBecauseVoted);
            }

            self.balances.insert(&from, &(msg_sender_balance - tokens));

            let to_balance = self.balance_of(to);
            self.balances.insert(&to, &(to_balance + tokens));
            self.env().emit_event(Transfer{
                from: Some(from), 
                to: to, 
                value: tokens
            });
            Ok(())
        }

        #[ink(message)]
        pub fn approve(&mut self, spender: AccountId, tokens: u128) -> Result<()>{
            let msg_sender: AccountId = self.env().caller();
            if self.balance_of(msg_sender) < tokens {
                return Err(Error::InsufficientBalance);
            }
    
            self.allowances.insert((msg_sender, spender), &tokens);
            self.env().emit_event(Approval {
                owner: msg_sender,
                spender: spender,
                value: tokens
            });
            Ok(())
        }

        #[ink(message)]
        pub fn initiate_voting(&mut self, option: u128) -> Result<()>{
            let caller: AccountId = self.env().caller();
            if (self.balance_of(caller) as f32) < self.total_supply as f32 * 0.1 {
                return Err(Error::InsufficientBalance);
            } else if self.is_voting_happening || self.get_current_timestemp() < self.voting_end_time {
                return Err(Error::VotingIsAlreadyOngoing);
            } else {
                self.voting_begin_time = self.get_current_timestemp();
                self.voting_end_time = self.get_current_timestemp() + self.time_to_vote;
                self.is_voting_happening = true;
                self.vote(option).map_err(|err: Error| ink::env::debug_println!("{:?}", err)).ok();
                self.voting_number+=1;
            }
            self.env().emit_event(VotingStartedTime{
                voting_begin_time: self.voting_begin_time,
                voting_number: self.voting_number
            });
            Ok(())
        }

        #[ink(message)]
        pub fn vote(&mut self, price: u128) -> Result<()>{
            let msg_sender: AccountId = self.env().caller();
            if !self.is_voting_happening ||
                self.env().block_timestamp() > self.voting_end_time || 
                self.env().block_timestamp() < self.voting_begin_time
            {
                return Err(Error::VotingIsNotOngoing);
            } else if (self.balance_of(msg_sender) as f32) < self.total_supply as f32 * 0.05 {
                return Err(Error::InsufficientBalance);
            } else if self.already_voted.get(msg_sender).unwrap_or(false) {
                return Err(Error::AccountAlreadyVoted);
            }

            self.votes.insert(price, &self.balance_of(msg_sender));
            self.already_voted.insert(msg_sender, &true);
            if self.votes.get(self.current_winner).unwrap_or(0) == 0 {
                self.current_winner = price;
            } else if self.votes.get(self.current_winner).unwrap_or(0) < self.votes.get(price).unwrap_or(0) {
                self.current_winner = price;
            }
            Ok(())
        }

        #[ink(message)]
        pub fn end_voting(&mut self) -> Result<()> {
            if self.env().block_timestamp() < self.voting_end_time {
                return Err(Error::TimeToVoteNotElapsed);
            } else if self.env().block_timestamp() < self.voting_begin_time || !self.is_voting_happening {
                return Err(Error::VotingIsNotOngoing);
            }
            self.token_price = self.current_winner;
            //setting all to default
            self.is_voting_happening = false;
            self.voting_begin_time = Default::default();
            self.voting_end_time = Default::default();
            self.votes = Mapping::default();
            self.already_voted = Mapping::default();
//add token price to the constructor
            Ok(())
        }

        #[ink(message, payable)]
        pub fn buy(&mut self, amount: u128) -> Result<()>{
            let msg_sender = self.env().caller();
            if self.already_voted.get(msg_sender).unwrap_or_default() {
                return Err(Error::AccountFrozenBecauseVoted);
            }
            if self.env().transferred_value() == amount * self.token_price {
                self.mint(amount).map_err(|err: Error| ink::env::debug_println!("{:?}", err)).ok();
                //counting fee
                let fee_to_take = (amount * self.fee) / self.fee_divider;
                //balance + amount - fee
                self.balances.insert(msg_sender, &(self.balance_of(msg_sender) + amount - fee_to_take));
                self.weekly_fee_to_burn += fee_to_take;
            } else {
                return Err(Error::ExactAmountOfWeiRequired);
            }
            Ok(())
        }

        #[ink(message, payable)]
        pub fn burn_fee_weekly (&mut self) -> Result<()> {
            let msg_sender = self.env().caller();
            if self.time_lapsed_for_fee_to_burn + 604800 < self.env().block_timestamp() {
                return Err(Error::TimeForFeeBurnHasNotLapsed);
            } 
            self.burn(self.weekly_fee_to_burn, msg_sender).map_err(|err: Error| ink::env::debug_println!("{:?}", err)).ok();
            self.weekly_fee_to_burn = 0;
            self.time_lapsed_for_fee_to_burn = self.env().block_timestamp();
            Ok(())
        }

        #[ink(message)]
        pub fn mint(&mut self, value: u128) -> Result<()> {
            self.total_supply += value;            
            Ok(())
        }

        #[ink(message, payable)]
        pub fn sell(&mut self, amount: u128) -> Result<()>{
            let msg_sender = self.env().caller();
            if self.already_voted.get(msg_sender).unwrap_or_default() {
                return Err(Error::AccountFrozenBecauseVoted);
            }
            //counting fee
            let fee_to_take = (amount * self.fee) / self.fee_divider;
            if self.balance_of(msg_sender) < amount + fee_to_take {
                return Err(Error::InsufficientBalance);
            } else {
                self.burn(amount, msg_sender).map_err(|err: Error| ink::env::debug_println!("{:?}", err)).ok();
                //balance + amount - fee
                self.balances.insert(msg_sender, &(self.balance_of(msg_sender) as u128 - amount as u128 - fee_to_take as u128));
                self.weekly_fee_to_burn += &fee_to_take;
            }
            Ok(())
        }

        #[ink(message)]
        pub fn burn(&mut self, value: u128, address: AccountId) -> Result<()> {
            self.total_supply = self.total_supply - value;
            self.balances.insert(address, &(self.balance_of(address) - value));
            Ok(())
        }

        //// Modifies the code which is used to execute calls to this contract address (`AccountId`).
        ////
        //// We use this to upgrade the contract logic. We don't do any authorization here, any caller
        //// can execute this method. In a production contract you would do some authorization here.
        #[ink(message)]
        pub fn set_code(&mut self, code_hash: [u8; 32]) {
            ink::env::set_code_hash(&code_hash).unwrap_or_else(|err| {
                panic!(
                    "Failed to `set_code_hash` to {:?} due to {:?}",
                    code_hash, err
                )
            });
            ink::env::debug_println!("Switched code hash to {:?}.", code_hash);
        }
    }

    #[cfg(test)]
    mod tests {    
        use super::*;

        fn default_accounts() -> ink::env::test::DefaultAccounts<Environment> {
            ink::env::test::default_accounts::<Environment>()
        }

        fn alice() -> AccountId {
            default_accounts().alice
        }

        fn bob() -> AccountId {
            default_accounts().bob
        }

        #[ink::test]
        fn returns_zero_balance(){
            let contract = Erc20::new(100);
            assert_eq!(contract.balance_of(bob()), 0);
        }

        #[ink::test]
        fn returns_total_supply() {
            let contract = Erc20::new(777);
            assert_eq!(contract.total_supply(), 777);
        }

        #[ink::test]
        fn returns_balance_of_msg_sender() {
            let contract = Erc20::new(100);
            assert_eq!(contract.balance_of(alice()), 100);
        }

        #[ink::test]
        fn transfer_happens(){
            let mut contract = Erc20::new(100);
            let accounts = 
                ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            
            // default balance of bob
            assert_eq!(contract.balance_of(accounts.bob), 0);
            // transfer method
            assert_eq!(contract.transfer(accounts.bob, 20), Ok(()));
            //eventual balance
            assert_eq!(contract.balance_of(accounts.bob), 20);
        }

        #[ink::test]
        fn transfer_error_when_insfficient_funds(){
            let mut contract = Erc20::new(0);
            let accounts = 
                ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();

            assert_eq!(contract.balance_of(accounts.bob), 0);
            assert!(contract.transfer(accounts.bob, 100).is_err());
            assert_eq!(contract.transfer(accounts.bob, 100), Err(Error::InsufficientBalance));
        }

        #[ink::test]
        fn transfer_from_happens(){
            let mut contract = Erc20::new(100);
            let accounts = 
                ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            // default balance of bob
            assert_eq!(contract.balance_of(accounts.bob), 0);
            assert_eq!(contract.approve(accounts.bob, 20), Ok(()));
            // transfer method
            assert_eq!(contract.transfer_from(accounts.alice, accounts.bob, 20), Ok(()));
            //eventual balance
            assert_eq!(contract.balance_of(accounts.bob), 20);
        }

        #[ink::test]
        fn transfer_from_error_when_insfficient_funds(){
            let mut contract = Erc20::new(0);
            let accounts = 
                ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();

            assert_eq!(contract.balance_of(accounts.bob), 0);
            assert!(contract.transfer_from(accounts.alice, accounts.bob, 100).is_err());
            assert_eq!(contract.transfer_from(accounts.alice, accounts.bob, 100), Err(Error::InsufficientBalance));
        }

        #[ink::test]
        fn transfer_from_error_insufficient_allowance(){
            let mut contract = Erc20::new(100);
            let accounts = 
                ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            assert_eq!(contract.approve(accounts.bob, 10), Ok(()));
            assert_eq!(contract.allowances(accounts.alice, accounts.bob), 10);
            assert_eq!(contract.transfer_from(accounts.alice, accounts.bob, 100), Err(Error::InsufficientAllowance));
        }


        #[ink::test]
        fn approve_happens(){
            let mut contract = Erc20::new(100);
            let accounts = 
                ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            assert_eq!(contract.approve(accounts.bob, 10), Ok(()));
            assert_eq!(contract.allowances(accounts.alice, accounts.bob), 10);
        }

        #[ink::test]
        fn approve_error(){
            let mut contract = Erc20::new(100);
            let accounts = 
                ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            assert!(contract.approve(accounts.bob, 110).is_err());
            assert_eq!(contract.approve(accounts.bob, 110), Err(Error::InsufficientBalance));
        }

        #[ink::test]
        fn initiate_voting_works(){
            let mut contract = Erc20::new(100);
            let accounts = 
                ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            let new_price: u128 = 50;
            //is_voting false
            assert_eq!(contract.is_voting_happening, false);
            //transfer 1 token to bob
            assert_eq!(contract.transfer(accounts.bob, 1), Ok(()));
            assert_eq!(contract.balance_of(accounts.bob), 1);
            //call the function, should return error
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
            assert_eq!(contract.initiate_voting(new_price), Err(Error::InsufficientBalance));
            //transfer 9 more tokens to bob
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
            assert_eq!(contract.transfer(accounts.bob, 9), Ok(()));
            assert_eq!(contract.balance_of(accounts.bob), 10);
            //voting end time has not began to run yet
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
            assert_eq!(contract.voting_end_time, 0);
            //set block timestamp to 1
            ink::env::test::set_block_timestamp::<ink::env::DefaultEnvironment>(1);
            // successful initiate voting
            assert_eq!(contract.initiate_voting(new_price), Ok(()));

            // ink::env::debug_println!("block time stamp: {}", ink::env::block_timestamp::<ink::env::DefaultEnvironment>()); 
            
            //is_voting true
            assert_eq!(contract.is_voting_happening, true);
            //time to vote began
            assert_eq!(contract.voting_end_time, contract.get_current_timestemp() + contract.time_to_vote);
            assert_eq!(contract.voting_begin_time, contract.get_current_timestemp());
        }

        #[ink::test]
        fn vote_accepted(){
            let mut contract = Erc20::new(100);
            let accounts = default_accounts();
            let mut new_price: u128 = 50;

            //call the vote function when voting is not happening
            assert_eq!(contract.transfer(accounts.charlie, 10), Ok(()));
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.charlie);
            assert_eq!(contract.vote(new_price), Err(Error::VotingIsNotOngoing));
            assert_eq!(contract.initiate_voting(new_price), Ok(()));
            assert_eq!(contract.current_winner, new_price);
            assert_eq!(contract.votes.get(50).unwrap_or(0), 10);
            
            //set bob as contract caller
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
            //call iniciate voting from bob's account
            assert_eq!(contract.initiate_voting(new_price), Err(Error::VotingIsAlreadyOngoing));
            assert_eq!(contract.balance_of(accounts.bob), 0);
            //call the vote function
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
            assert_eq!(contract.vote(new_price), Err(Error::InsufficientBalance));
            
            //transfer 15 tokens to bob
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
            assert_eq!(contract.transfer(accounts.bob, 15), Ok(()));
            assert_eq!(contract.balance_of(accounts.bob), 15);

            //change caller to bob
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
            //he has not voted yet
            assert_eq!(contract.already_voted.get(accounts.bob).unwrap_or_default(), false);
            //call the vote function
            new_price = 60;
            assert_eq!(contract.vote(new_price), Ok(()));
            //check the mapping: price option voted for == account voting power
            assert_eq!(contract.votes.get(60), Some(contract.balance_of(bob())));
            assert_eq!(contract.current_winner, 60);
            assert_eq!(contract.votes.get(60).unwrap_or(0), 15);
            //bob has already voted
            assert_eq!(contract.already_voted.get(accounts.bob).unwrap_or_default(), true);
            assert_eq!(contract.vote(new_price), Err(Error::AccountAlreadyVoted));

            //transfer 5 to charlie
            assert_eq!(contract.balance_of(accounts.charlie), 10);
            //with transfer
            assert_eq!(contract.transfer(accounts.charlie, 5), Err(Error::AccountFrozenBecauseVoted));
            assert_eq!(contract.approve(accounts.charlie, 5), Ok(()));
            assert_eq!(contract.allowances(accounts.bob, accounts.charlie), 5);
            //with transfer_from
            assert_eq!(contract.transfer_from(accounts.bob, accounts.charlie, 5), Err(Error::AccountFrozenBecauseVoted));
            assert_eq!(contract.balance_of(accounts.bob), 15);
            assert_eq!(contract.balance_of(accounts.charlie), 10);
            assert_eq!(contract.current_winner, 60);

        }

        #[ink::test]
        pub fn end_voting_success(){
            let mut contract = Erc20::new(100);
            let accounts: ink::env::test::DefaultAccounts<ink::env::DefaultEnvironment> = default_accounts();
            let new_price: u128 = 50;

            //call the vote function when voting is not happening
            assert_eq!(contract.transfer(accounts.bob, 10), Ok(()));
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
            assert_eq!(contract.vote(new_price), Err(Error::VotingIsNotOngoing));
            assert_eq!(contract.end_voting(), Err(Error::VotingIsNotOngoing));
            ink::env::test::set_block_timestamp::<ink::env::DefaultEnvironment>(1);
            assert_eq!(contract.initiate_voting(new_price), Ok(()));
            
            ink::env::debug_println!("voting_end_time: {}", contract.voting_end_time);

            //checks the time has elapsed
            assert_eq!(contract.end_voting(), Err(Error::TimeToVoteNotElapsed));
            //logic for time travel
            ink::env::debug_println!("block number: {}", ink::env::block_number::<ink::env::DefaultEnvironment>());
            ink::env::debug_println!("block time stamp: {}", ink::env::block_timestamp::<ink::env::DefaultEnvironment>()); 
            ink::env::test::advance_block::<ink::env::DefaultEnvironment>();
            ink::env::test::set_block_timestamp::<ink::env::DefaultEnvironment>(contract.voting_end_time);
            ink::env::debug_println!("block number: {}", ink::env::block_number::<ink::env::DefaultEnvironment>());
            ink::env::debug_println!("block time stamp: {}", ink::env::block_timestamp::<ink::env::DefaultEnvironment>());
            assert_eq!(contract.end_voting(), Ok(()));
            ink::env::test::advance_block::<ink::env::DefaultEnvironment>();
            ink::env::debug_println!("block number: {}", ink::env::block_number::<ink::env::DefaultEnvironment>());
            ink::env::debug_println!("block time stamp: {}", ink::env::block_timestamp::<ink::env::DefaultEnvironment>());

            assert_eq!(contract.voting_end_time, 0);
            assert_eq!(contract.voting_begin_time, 0);
            ink::env::debug_println!("contract.already_voted.get(accounts.bob): {}", contract.already_voted.get(accounts.bob).unwrap());

            // assert_eq!(contract.already_voted.get(accounts.bob).unwrap_or(false), false);
            assert_eq!(contract.is_voting_happening, false);
            assert_eq!(contract.current_winner, 50);
            assert_eq!(contract.votes.get(50).unwrap_or(0), 10);
        }

        #[ink::test]
        pub fn buy_works (){
            let mut contract = Erc20::new(100);
            let accounts: ink::env::test::DefaultAccounts<ink::env::DefaultEnvironment> = default_accounts();

    
            //total supply and contract balance are in their initial values
            assert_eq!(ink::env::balance::<ink::env::DefaultEnvironment>(), 1000000);
            assert_eq!(contract.total_supply, 100);

            // assert_eq!(contract.transfer(accounts.bob, 50), Ok(()));

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
            //transfer of 50 ETH to the contract
            ink::env::test::transfer_in::<ink::env::DefaultEnvironment>(50);
            //incorrect amount passed to the buy method
            assert_eq!(contract.buy(8), Err(Error::ExactAmountOfWeiRequired));
            //correct amount passed to the buy method
            assert_eq!(contract.buy(10), Ok(()));
            //total supply +10
            assert_eq!(contract.total_supply, 110);
            //bob + 10
            assert_eq!(contract.balance_of(accounts.bob), 10);
            //check that contract has reveiced 50 ETH
            assert_eq!(ink::env::balance::<ink::env::DefaultEnvironment>(), 1000050);            
        }

        #[ink::test]
        pub fn sell_works () {
            let mut contract = Erc20::new(100);
            let accounts: ink::env::test::DefaultAccounts<ink::env::DefaultEnvironment> = default_accounts();

            //bob's try with 0 balance
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
            assert_eq!(contract.sell(10), Err(Error::InsufficientBalance));
            //transfer tokens to bob
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
            assert_eq!(contract.transfer(accounts.bob, 11), Ok(()));
            //second bob's try
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
            assert_eq!(contract.sell(10), Ok(()));
            
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
            
            assert_eq!(contract.total_supply, 90);
            
            assert_eq!(contract.balance_of(accounts.alice), 90);
            // assert_eq!(contract.balance_of(accounts.bob), 0); 
        }

        #[ink::test]
        pub fn burn_fee_works () {
            let mut contract = Erc20::new(100);
            let accounts: ink::env::test::DefaultAccounts<ink::env::DefaultEnvironment> = default_accounts();

            assert_eq!(contract.burn_fee_weekly(), Ok(()));
        }
    }
}
