// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import clsx from 'clsx';
import { type ReactNode } from 'react';

export interface ListItemProps {
    active?: boolean;
    children: ReactNode;
    onClick?(): void;
}

export function ListItem({ active, children, onClick }: ListItemProps) {
    return (
        <li className="list-none">
            <button
                type="button"
                className={clsx(
                    'border-1 block w-full cursor-pointer rounded-md border-solid px-3 py-2 text-left text-body',
                    active
                        ? 'border-sui-grey-50 bg-sui-grey-45 font-semibold text-sui-grey-90 shadow-sm'
                        : 'border-transparent bg-white font-medium text-sui-grey-80'
                )}
                onClick={onClick}
            >
                {children}
            </button>
        </li>
    );
}

export interface VerticalListProps {
    children: ReactNode;
}

export function VerticalList({ children }: VerticalListProps) {
    return <ul className="m-0 flex flex-col gap-1 p-0">{children}</ul>;
}
