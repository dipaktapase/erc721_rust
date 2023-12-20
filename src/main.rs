#[ink::contract]

mod erc721 {
    use ink::storage::Mapping;
    use scale::{Decode, Encode};

    pub type TokenId = u32; // Type is used to ceate an alias to another data type

    #[ink(storage)] // attribute ink storage
    #[derive(Default)] // this is a convinient way to set initial state of contract, It signifies initially there is no token

    pub struct Erc721 {
        token_owner: Mapping<TokenId, AccountId>,
        token_approvals: Mapping<TokenId, AccountId>,
        owned_tokens_count: Mapping<TokenId, u32>,
        operator_approvals: Mapping<(AccountId, AccountId), ()>,
    }

    // error enum
    #[derive(Encode, Decode, Debug, PartialEq, Eq, Copy, Clone)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))] // Condition compilation attribute

    pub enum Error {
        NotOwner,
        NotApproved,
        TokenExists,
        TokenNotFound,
        CannotInsert,
        CannotFetchValue,
        NotAllowed,
    }

    // Event Definations
    #[ink(event)]
    pub struct Transfer {
        #[ink(topic)]
        from: Option<AccountId>,
        #[ink(topic)]
        to: Option<AccountId>,
        #[ink(topic)]
        id: TokenId,
    }

    #[ink(event)]
    pub struct Approval {
        #[ink(topic)]
        from: AccountId,
        #[ink(topic)]
        to: AccountId,
        #[ink(topic)]
        id: TokenId,
    }

    #[ink(event)]
    pub struct ApprovalForAll {
        #[ink(topic)]
        owner: AccountId,
        #[ink(topic)]
        operator: AccountId,
        approved: bool,
    }

    // Constructor -> to initialize a new instance
    #[ink(constructor)]
    pub fn new() -> Self {
        Default::default()
    }

    #[ink(message)]
    #[ink(message)] // This indicates that function can be called externally
    pub fn balance_of(&self, owner: AccountId) -> u32 {
        self.balance_of_or_zero(&owner)
    }

    #[ink(message)]
    pub fn owner_of(&self, id: TokenId) -> Option<AccountId> {
        self.token_owner.get(id)
    }

    // Transfer the token from the caller to the given destination
    #[ink(message)]
    pub fn transfer(&mut self, destination: AccountId, id: TokenId) -> Result<(), Error> {
        let caller = self.env().caller();
        self.transfer_token_from(&caller, &destination, id)?; // ? => It's an error propogation operator, It is used for concise error handling in rust
        Ok(())
    }

    #[ink(message)]
    pub fn transfer_from(
        &mut self,
        from: AccountId,
        to: AccountId,
        id: TokenId,
    ) -> Result<(), Error> {
        self.transfer_token_from(&from, &to, id)?;
        Ok(())
    }

    // Returns a total number of tokens from an account
    fn balance_of_or_zero(&self, of: &AccountId) -> u32 {
        self.owned_tokens_count.get(of).unwrap_or(0) // .unwrap_or(0) => it says if the option is Some then give me the contained value but if it's None -> return zero
    }

    fn transfer_token_from(&mut self, from: &AccountId, to: AccountId, id: TokenId) -> Result<(), Error> { // Result<(), Error> this return type is usually used to error handling, with Ok and Err
        let caller = self.env().caller();
        if !self.exists(id) {
            return Err(Error::TokenNotFound);
        };
        
        if !self.approved_or_owner(Some(caller), id) { // Some is a varient of option enum in rust
            return  Err(Error::NotApproved); // Err represent failure and containing an error
        };
        self.clear_approval(id);
        self.remove_token_from(from, to);
        self.add_token_to(to, id);
        self.env().emit_event(Transfer {
            from: Some(*from),
            to: Some(*to),
            id,
        });
        Ok(()) // Ok represent success in contract, () => is rust unit type indicating that no actual value returning on success
    }

    // Returns true if 'token id' exist or false if it does not.
    fn exists(&self, id: TokenId) -> bool {
        self.token_owner_contains(id)
    }

    // Returns true if the 'AccountId' 'from' is the owner of the token id
    // or it has been approved on behalf of the token id owner
    fn approved_or_owner(&self, from: Option<AccountId>, id: TokenId) -> bool {
        let owner = self.owner_of(id);
        from != Some(AccountId::from([0x0; 32])) // [0x0; 32] is zero or null address. 
            && (from == owner 
                || from == self.token_approvals.get(id)
                || self.approved_for_all(
                    owner.expect("Error with AccountId"),
                    from.expect("Error with AccountId"),
                ))
    }

    #[ink(message)]
    pub fn approve(&mut self, to: AccountId, id: TokenId) -> Result<(), Error> {
        self.approve_for(&to, id)?;
        Ok(())
    }

    // Approves of Disapproves the operator for all tokens of the caller
    #[ink(message)]
    pub fn set_approval_for_all(&mut self, to: AccountId, approved: bool) -> Result<(), Error> {
        self.approve_for_all(to, approved)?;
        Ok(())
    }
    
    // Approve the passed 'AccountId' to transfer the specified token on behalf of the message sender
    fn approve_for(&mut self, to: AccountId, id: TokenId) -> Result<(), Error> {
        let caller = self.env().caller();
        let owner = self.owner_of(id);
        if !(owner == Some(caller) || self.approved_for_all(owner.expect("Error with AccountId"), caller)) {
            return Err(Error::NotAllowed)
        };

        if *to == AccountId::from([0x0, 32]) {
            return  Err(Error::NotAllowed);
        };

        if self.token_approvals.contains(id) {
            return Err(Error::CannotInsert)
        } else {
            self.token_approvals.insert(id, to);
        }

        self.env().emit_event(Approval {
            from: caller,
            to: *to,
            id,
        });

        Ok(())
    }

    fn approve_for_all(&mut self, to: AccountId, approved: bool) -> Result<(), Error> {
        let caller = self.env().caller();
        if to == caller {
            return Err(Erron::NotAllowed)
        }
        self.env().emit_event(ApprovalForAll {
            owner: caller,
            operator: to,
            approved,
        });

        if approved {
            self.operator_approvals.insert((&caller, &to), &()); // &() is show no value or nothing
        } else {
            self.operator_approvals.remove((&caller, &to));
        }

        Ok(())
    }

    // Create a new token
    #[ink(message)]
    pub fn mint(&mut self, id: TokenId) -> Result<(), Error> {
        let caller = self.env().caller();
        self.add_token_to(caller, id)?;
        self.env().emit_event(Transfer {
            from: Some(AccountId::from([0x0, 32])),
            to: Some(caller),
            id,
        });
        Ok(())
    }

    // Deleting an existing token. Only the owner can burn the token 
    #[ink(message)]
    pub fn burn(&mut self, id: TokenId) -> Result<(), Error> {
        let caller = self.env().caller();

        // We are destruring. We are extracting the token_owner and owner_tokes_from fileds from the struct
        // .. two dots indicate the other field in the struct can be ingored
        let Self {
            token_owner,
            owned_tokens_count,
            ..
        } = self;

        let owner = token_owner.get(id).ok_or(Error::TokenNotFound)?;
        if owner != caller {
            return Err(Error::NotOwner)
        };

        let count = owned_tokens_count
        .get(caller)
        .map(|c| c - 1) // Closures, they are defined inside bars |
        .ok_or(Error::CannotFetchValue)?;
        owned_tokens_count.insert(caller, &count);
        token_owner.remove(id);

        self.env().emit_event(Transfer {
            from: Some(caller),
            to: Some(AccountId::from([0x0; 32])),
            id,
        });

        Ok(())
    }

    // Adds the token 'id' to the 'to' AccountId
    fn add_token_to(&mut self, to: &AccountId, id: TokenId) -> Result<(), Error> {
        let Self {
            token_owner,
            owned_tokens_count,
            ..
        } = self;

        if token_owner.contains(id) {
            return Err(Error::TokenExists);
        }

        if *to == AccountId::from([0x0, 32]) {
            return Err(Error::NotAllowed)
        };

        let count = owned_tokens_count.get(to).map(|c| c + 1).unwrap_or(1);

        owned_tokens_count.insert(to, &count);
        token_owner.insert(id, to);

        Ok(())
    }

    // Remove tokes 'id' from the owner
    fn remove_token_from(&mut self, from: &AccountId, id: TokenId) -> Result<(), Error> {
        let Self {
            token_owner,
            owned_tokens_count,
            ..
        } = self;

        if !token_owner.contains(id) {
            return Err(Error::TokenNotFound)
        }

        let count = owned_tokens_count.get(from).map(|c| c - 1).ok_or(Error::CannotFetchValue)?;
        owned_tokens_count.insert(from, &count);
        token_owner.remove(id);

        Ok(())
    }

    // Remove existing approval from token 'id'
    fn clear_approval(&mut self, id: TokenId) {
        self.token_approvals.remove(id);
    }
}

fn main() {}