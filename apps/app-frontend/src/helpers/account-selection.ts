type AccountLike = {
	account_id: string
	account_type: string
}

export function preferredOnlineAccountId<T extends AccountLike>(
	selectedAccountId: string | undefined,
	accounts: readonly T[],
): string | undefined {
	if (!selectedAccountId) return selectedAccountId
	const selectedAccount = accounts.find((account) => account.account_id === selectedAccountId)
	if (selectedAccount?.account_type !== 'offline') return selectedAccountId
	return accounts.find((account) => account.account_type !== 'offline')?.account_id
}
