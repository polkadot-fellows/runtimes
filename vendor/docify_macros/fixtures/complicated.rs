mod tests {
    mod submodule {
        #[docify::export]
        #[test]
        fn successful_unstake() {
            ExtBuilder::default().build_and_execute(|| {
                ErasToCheckPerBlock::<T>::put(BondingDuration::get() + 1);
                CurrentEra::<T>::put(BondingDuration::get());
                // register for fast unstake
                assert_ok!(FastUnstake::register_fast_unstake(RuntimeOrigin::signed(1)));
                assert_eq!(Queue::<T>::get(1), Some(Deposit::get()));
                // process on idle
                next_block(true);
                // assert queue item has been moved to head
                assert_eq!(Queue::<T>::get(1), None);
                // assert head item present
                assert_eq!(
                    Head::<T>::get(),
                    Some(UnstakeRequest {
                        stashes: bounded_vec![(1, Deposit::get())],
                        checked: bounded_vec![3, 2, 1, 0]
                    })
                );
                next_block(true);
                assert_eq!(Head::<T>::get(), None,);
                assert_eq!(
                    fast_unstake_events_since_last_call(),
                    vec![
                        Event::BatchChecked {
                            eras: vec![3, 2, 1, 0]
                        },
                        Event::Unstaked {
                            stash: 1,
                            result: Ok(())
                        },
                        Event::BatchFinished { size: 1 }
                    ]
                );
                assert_unstaked(&1);
            });
        }
    }
}
