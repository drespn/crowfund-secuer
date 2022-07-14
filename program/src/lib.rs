

use borsh::{BorshSerialize, BorshDeserialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint,
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
    rent::Rent,
    sysvar::Sysvar,
};

fn process_instruction(
    // program id is nothing but the id of this program on the solana network.
    program_id: &Pubkey,
    // When we invoke our program we can 
    // give meta data of all the account we 
    // want to work with.
    // As you can see it is a array of AccountInfo.
    // We can provide as many as we want.
    accounts: &[AccountInfo],
    // This is the data we want to process our instruction for.
    // It is a list of 8 bitunsigned integers(0..255).
    instruction_data: &[u8],
    
    // Here we specify the return type.
    // If you know a little bit of typescript. 
    // This was of writing types and returns types might we familiar to you.
) -> ProgramResult {
    //First run a check if we got any instruction data at all
     if instruction_data.len() == 0 {
        return Err(ProgramError::InvalidInstructionData);
     }

     if instruction_data[0] == 0{
        return create_campaign(
            program_id,
            accounts,
            //pass a reference to the rest of instruction data
            &instruction_data[1..],
        );
     } else if instruction_data[0] == 1 {
        return widthdrawal(
            program_id,
            accounts,
            &instruction_data[1..],
        );
     } else if instruction_data[0] == 2 {
        return donate(
            program_id,
            accounts,
            &instruction_data[1..],
        );
     }

    // If instruction_data doesn't match we give an error.
    msg!("Didn't find the entrypoint required");
    Err(ProgramError::InvalidInstructionData)

    /*
    And then since we can't return null in Rust we pass `Ok(())` to make it compile
    It means the program executed successfully.
    */
}

entrypoint!(process_instruction);


fn create_campaign(
    programid: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult{

    let account_info_iter = &mut accounts.iter();

    //new pubkey for campaign
    let campaign_acc = next_account_info(account_info_iter)?;

    //get the initializers account
    let initializer_acc = next_account_info(account_info_iter)?;

    //CHECK: that the person who started it is the signer of tx
    if !initializer_acc.is_signer {
        return Err(ProgramError::IncorrectProgramId);
    }
    //CHECK: that program id (the campaign program owns this program)
    if campaign_acc.owner != programid {
        msg!("Program id needs to own campaign account");
        return Err(ProgramError::IncorrectProgramId);
    }

    //create CampaignDetails instance
    let mut campaign_info = CampaignDetails::try_from_slice(&instruction_data)?;

    // Now I want that for a campaign created the only admin should be the one who created it.
    // You can add additional logical here to check things like
    // The image url should not be null
    // The name shouldn't be smaller than some specific length...
    if campaign_info.admin != *initializer_acc.key {
        msg!("Admin is not the same as creator");
        return Err(ProgramError::InvalidInstructionData);
    }

    //Check that the campaign_acc is rent exempt
    let rent = &Rent::from_account_info(campaign_acc)?;
    if !rent.is_exempt(campaign_acc.lamports(), campaign_acc.data_len()) {
        return Err(ProgramError::AccountNotRentExempt);
    }

    //set amount donated to zero
    campaign_info.amount_donated = 0;

    campaign_info.serialize(&mut &mut campaign_acc.data.borrow_mut()[..])?;

    Ok(())
}



#[derive(BorshSerialize, BorshDeserialize, Debug)]
struct widthdrawalRequest{
    pub amount: u64,
}
fn widthdrawal(
    programid: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult{

    //start by getting both accounts
    let account_info_iter = &mut accounts.iter();

    //new pubkey for campaign
    let campaign_acc = next_account_info(account_info_iter)?;

    //get the initializers account
    let admin_acc = next_account_info(account_info_iter)?;

    //checks
    if !admin_acc.is_signer {
        return Err(ProgramError::IncorrectProgramId);
    }

    if campaign_acc.owner != programid {
        msg!("Program needs to own campaign account.");
        return Err(ProgramError::IncorrectProgramId);
    }

    let campaign_data = CampaignDetails::try_from_slice(*campaign_acc.data.borrow())?;

    //check admin of campaign
    if campaign_data.admin != *admin_acc.key {
        msg!("Admin should be the owner of campaign.");
        return Err(ProgramError::IncorrectProgramId);
    }

    //try to make widthdrawal
    let input_data = widthdrawalRequest::try_from_slice(&instruction_data)?;



    //check that account has enough rent
    let rent_exemption = Rent::get()?.minimum_balance(campaign_acc.data_len());
    if campaign_acc.lamports() - rent_exemption < input_data.amount {
        msg!("Not enough funds for widthdrawal.");
        return Err(ProgramError::InsufficientFunds);
    }

    //trasnfer the funds
    //decrease the campaigns account
    **campaign_acc.try_borrow_mut_lamports()? -= input_data.amount;
    **admin_acc.try_borrow_mut_lamports()? += input_data.amount;

    Ok(())
}
fn donate(
    programid: &Pubkey,
    accounts: &[AccountInfo],
    _instruction_data: &[u8],
) -> ProgramResult{

    //need to create PDA for the donator in order for contract to be able 
    //to decrease the balance of donator

    //3 acccounts
    let account_iter = &mut accounts.iter();
    //1.the campaign account
    let campaign_acc = next_account_info(account_iter)?;
    //2.PDA of donator
    let pda_donator = next_account_info(account_iter)?;
    //3. Donator account to sign it
    let donator = next_account_info(account_iter)?;

    if campaign_acc.owner != programid {
        msg!("Program should be owner of campaign.");
        return Err(ProgramError::IncorrectProgramId);
    }

    if pda_donator.owner != programid {
        msg!("Program should own donator derived account.");
        return Err(ProgramError::IncorrectProgramId);
    }

    if !donator.is_signer {
        msg!("Donator should have signed transaction.");
        return Err(ProgramError::IncorrectProgramId);
    }
    //get the data from campaign account
    let mut campaign_data = CampaignDetails::try_from_slice(*campaign_acc.data.borrow())?;
    //fucking borrow() method
    campaign_data.amount_donated += **pda_donator.lamports.borrow();

    //do transaction
    **campaign_acc.try_borrow_mut_lamports()? += **pda_donator.lamports.borrow();
    //close account
    **pda_donator.try_borrow_mut_lamports()? = 0;

    //reserialize data
    campaign_data.serialize(&mut &mut campaign_acc.data.borrow_mut()[..])?;


    Ok(())
}

//create campaign structure
#[derive(BorshSerialize, BorshDeserialize, Debug)]
struct CampaignDetails {
    pub admin: Pubkey,
    pub name: String,
    pub description: String,
    pub image_link: String,
    /// we will be using this to know the total amount 
    /// donated to a campaign.
    pub amount_donated: u64,
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
