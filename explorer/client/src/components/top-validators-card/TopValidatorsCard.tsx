// Copyright (c) 2022, Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { useContext, useEffect, useState } from 'react';

import Longtext from '../../components/longtext/Longtext';
import TableCard from '../../components/table/TableCard';
import TabFooter from '../../components/tabs/TabFooter';
import Tabs from '../../components/tabs/Tabs';
import { NetworkContext } from '../../context';
import {
    getTabFooter,
    getValidatorState,
    processValidators,
    sortValidatorsByStake,
    stakeColumn,
    ValidatorLoadFail,
    type ValidatorState,
} from '../../pages/validators/Validators';
import { mockState } from '../../pages/validators/mockData';
import theme from '../../styles/theme.module.css';

import styles from './TopValidatorsCard.module.css';

export const STATE_DEFAULT: ValidatorState = {
    delegation_reward: 0,
    epoch: 0,
    id: { id: '', version: 0 },
    parameters: {
        type: '0x2::sui_system::SystemParameters',
        fields: {
            max_validator_candidate_count: 0,
            min_validator_stake: BigInt(0),
        },
    },
    storage_fund: 0,
    treasury_cap: {
        type: '',
        fields: {},
    },
    validators: {
        type: '0x2::validator_set::ValidatorSet',
        fields: {
            delegation_stake: BigInt(0),
            active_validators: [],
            next_epoch_validators: [],
            pending_removals: '',
            pending_validators: '',
            quorum_stake_threshold: BigInt(0),
            validator_stake: BigInt(0),
        },
    },
};

export const TopValidatorsCardStatic = (): JSX.Element => {
    return <TopValidatorsCard state={mockState as ValidatorState} />;
};

export const TopValidatorsCardAPI = (): JSX.Element => {
    const [showObjectState, setObjectState] = useState(STATE_DEFAULT);
    const [loadState, setLoadState] = useState('pending');
    const [network] = useContext(NetworkContext);
    useEffect(() => {
        getValidatorState(network)
            .then((objState: ValidatorState) => {
                setObjectState(objState);
                setLoadState('loaded');
            })
            .catch((error: any) => {
                console.log(error);
                setLoadState('fail');
            });
    }, [network]);

    if (loadState === 'loaded') {
        return <TopValidatorsCard state={showObjectState as ValidatorState} />;
    }
    if (loadState === 'pending') {
        return <div className={theme.pending}>loading validator info...</div>;
    }
    if (loadState === 'fail') {
        return <ValidatorLoadFail />;
    }

    return <div>"Something went wrong"</div>;
};

function TopValidatorsCard({ state }: { state: ValidatorState }): JSX.Element {
    const totalStake = state.validators.fields.validator_stake;
    // sort by order of descending stake
    sortValidatorsByStake(state.validators.fields.active_validators);

    const validatorsData = processValidators(
        state.validators.fields.active_validators,
        totalStake
    );

    // map the above data to match the table combine stake and stake percent
    const tableData = {
        data: validatorsData.map((validator) => ({
            name: validator.name,
            stake: stakeColumn(validator),
            delegation: validator.delegation_count,
            position: validator.position,
        })),
        columns: [
            {
                headerLabel: '#',
                accessorKey: 'position',
            },
            {
                headerLabel: 'Name',
                accessorKey: 'name',
            },
            {
                headerLabel: 'STAKE',
                accessorKey: 'stake',
            },
            {
                headerLabel: 'Delegators',
                accessorKey: 'delegation',
            },
        ],
    };

    const tabsFooter = getTabFooter(validatorsData.length);

    return (
        <div className={styles.validators}>
            <Tabs selected={0}>
                <div title="Top Validators">
                    <TableCard tabledata={tableData} />
                    <TabFooter stats={tabsFooter.stats}>
                        <Longtext
                            text=""
                            category="validators"
                            isLink={true}
                            isCopyButton={false}
                            /*showIconButton={true}*/
                            alttext="More Validators"
                        />
                    </TabFooter>
                </div>
                <div title=""></div>
            </Tabs>
        </div>
    );
}

export default TopValidatorsCard;
