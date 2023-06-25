#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

use frame_support::{
	codec::{Decode, Encode},
	inherent::Vec,
	sp_runtime::RuntimeDebug,
};
use scale_info::TypeInfo;

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum Votes {
	Yes,
	No,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct Vote<AccountId> {
	total_yes: Vec<AccountId>,
	total_no: Vec<AccountId>,
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
	}

	#[pallet::storage]
	#[pallet::getter(fn memberrequestedfordao)]
	pub type MemberRequestedForDao<T: Config> = StorageValue<_, Vec<T::AccountId>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn daousers)]
	pub type DaoUsers<T: Config> = StorageValue<_, Vec<T::AccountId>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn proposal)]
	pub type Proposal<T: Config> =
		StorageMap<_, Identity, T::Hash, Vote<T::AccountId>, OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn voted_users)]
	pub type VotedUsers<T: Config> =
	StorageValue<_, Vec<T::AccountId>, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		MemberRequestedToJoin {
			who: T::AccountId,
		},
		MemberAddedToDao {
			who: T::AccountId,
		},
		ProposedProposal {
			who: T::AccountId,
			proposal_id: T::Hash,
		},
		ProposalVoted {
			who: T::AccountId,
			proposal_id: T::Hash,
			recent_vote: Votes,
			total_yes: Vec<T::AccountId>,
			total_no: Vec<T::AccountId>,
		},
		ApprovedProposal {
			proposal_id: T::Hash,
		},
		ProposalNotApproved {
			proposal_id: T::Hash,
		},
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		MemberAlreadyRequested,
		MemberAlreadyPresentInDao,
		MemberNotPresentInDao,
		InvalidProposal,
		MemberNotRequested,
		DuplicateVote,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Any member can request to join the dao member group
		#[pallet::call_index(0)]
		#[pallet::weight(10_000)]
		pub fn request_to_join(origin: OriginFor<T>) -> DispatchResult {
			let who = ensure_signed(origin.clone())?;

			// Check if it also requested..
			let mut all_members = MemberRequestedForDao::<T>::get();
			let index = all_members
				.binary_search(&who)
				.err()
				.ok_or(Error::<T>::MemberAlreadyRequested)?;

			all_members.insert(index, who.clone());
			MemberRequestedForDao::<T>::put(all_members);

			Self::deposit_event(Event::<T>::MemberRequestedToJoin { who });

			Ok(())
		}

		/// Add the requested member in the dao members group.
		/// Only Sudo can perform this operation
		#[pallet::call_index(1)]
		#[pallet::weight(10_000)]
		pub fn add_requested_user_into_dao_member(
			origin: OriginFor<T>,
			who: T::AccountId,
		) -> DispatchResult {
			// ensure_root(origin)?;

			// Check if who is present in the dao_member group already.
			let mut all_dao_member = DaoUsers::<T>::get();
			let _index = all_dao_member
				.binary_search(&who)
				.err()
				.ok_or(Error::<T>::MemberAlreadyPresentInDao)?;

			// Check if member is not present in member request storage.
			let mut all_requested_members = MemberRequestedForDao::<T>::get();
			let index = all_requested_members
				.binary_search(&who)
				.ok()
				.ok_or(Error::<T>::MemberNotRequested)?;

			all_dao_member.insert(index, who.clone());

			DaoUsers::<T>::put(all_dao_member);

			// Remove this member from MemberRequestedForDao storage.
			all_requested_members.remove(index);
			MemberRequestedForDao::<T>::put(all_requested_members);

			Self::deposit_event(Event::<T>::MemberAddedToDao { who });

			Ok(())
		}

		/// Anyone can propose the proposal.

		#[pallet::call_index(2)]
		#[pallet::weight(10_000)]
		pub fn propose_proposal(origin: OriginFor<T>, proposal_id: T::Hash) -> DispatchResult {
			let who = ensure_signed(origin.clone())?;

			let votes = Vote { total_yes: Vec::new(), total_no: Vec::new() };

			// Initialize a new proposal
			Proposal::<T>::insert(proposal_id, votes);

			Self::deposit_event(Event::ProposedProposal { who, proposal_id });
			Ok(())
		}

		/// Only Dao members are allowed to vote on a proposal.
		///! Origin should be signed.
		#[pallet::call_index(3)]
		#[pallet::weight(10_000)]
		pub fn approve_proposal(
			origin: OriginFor<T>,
			proposal_id: T::Hash,
			approve: Votes,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			let all_dao_users = DaoUsers::<T>::get();

			// Check who is present in the dao or not.
			ensure!(all_dao_users.contains(&who), Error::<T>::MemberNotPresentInDao);

			// Prevent double voting
			let mut all_users = VotedUsers::<T>::get();
			ensure!(!all_users.contains(&who), Error::<T>::DuplicateVote);
			let current_user = who.clone();

			// Caste vote on a proposal
			match approve {
				Votes::Yes => {
					Proposal::<T>::mutate(&proposal_id, |info| {
						let total_votes = info.as_mut().unwrap();
						let total_yes_votes = &mut total_votes.total_yes;
						let total_no_votes = &mut total_votes.total_no;

						let _ = &total_yes_votes.push(who.clone());

						Self::deposit_event(Event::<T>::ProposalVoted {
							who,
							proposal_id,
							recent_vote: approve,
							total_yes: total_yes_votes.clone(),
							total_no: total_no_votes.clone(),
						});
					});
				},
				Votes::No => {
					Proposal::<T>::mutate(&proposal_id, |info| {
						let total_votes = info.as_mut().unwrap();
						let total_yes_votes = &mut total_votes.total_yes;
						let total_no_votes = &mut total_votes.total_no;

						let _ = &total_no_votes.push(who.clone());

						Self::deposit_event(Event::<T>::ProposalVoted {
							who,
							proposal_id,
							recent_vote: approve,
							total_yes: total_yes_votes.clone(),
							total_no: total_no_votes.clone(),
						});
					});
				},
			}
			all_users.push(current_user.clone());
			VotedUsers::<T>::put(all_users);

			Ok(())
		}

		///! origin should be signed
		///! If 2/3 votes are in favour of proposal then the proposal will approve.
		#[pallet::call_index(4)]
		#[pallet::weight(10_000)]
		pub fn check_status_of_proposal(
			origin: OriginFor<T>,
			proposal_id: T::Hash,
		) -> DispatchResult {
			ensure_signed(origin)?;
			let all_dao_user = DaoUsers::<T>::get();
			let threshold = (all_dao_user.len() as u32 * 2) / 3;

			let all_votes = Proposal::<T>::get(proposal_id).ok_or(Error::<T>::InvalidProposal)?;

			let yes_votes = all_votes.total_yes.len() as u32;

			if yes_votes >= threshold {
				Self::deposit_event(Event::<T>::ApprovedProposal { proposal_id })
			} else {
				Self::deposit_event(Event::<T>::ProposalNotApproved { proposal_id })
			}
			Ok(())
		}
	}
}
