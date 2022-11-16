// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { ReactComponent as PauseIcon } from './icons/pause.svg';
import { ReactComponent as PlayIcon } from './icons/play.svg';

export interface PlayPauseProps {
    paused?: boolean;
    onChange(): void;
}

// TODO: Create generalized `IconButton` component:
export function PlayPause({ paused, onChange }: PlayPauseProps) {
    return (
        <button
            type="button"
            aria-label={paused ? 'Paused' : 'Playing'}
            onClick={onChange}
            className="cursor-pointer border-none bg-transparent text-sui-grey-60 hover:text-sui-grey-80"
        >
            {paused ? <PlayIcon /> : <PauseIcon />}
        </button>
    );
}
