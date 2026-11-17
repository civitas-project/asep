// Package ledger provides core financial accounting domain contracts.
package ledger

import (
	"context"
	"errors"
	"time"
)

var (
	// ErrAccountNotFound indicates the requested account ID does not exist in the ledger store.
	ErrAccountNotFound = errors.New("ledger: account not found")

	// ErrNegativeAmount indicates the mutation amount requested was zero or negative.
	ErrNegativeAmount = errors.New("ledger: mutation amount must be positive")

	// ErrInsufficientFunds indicates the account cleared balance cannot support the debit.
	ErrInsufficientFunds = errors.New("ledger: insufficient funds")
)

// AccountState represents the snapshot of a ledger account.
type AccountState struct {
	ID        string
	Balance   int64
	UpdatedAt time.Time
}

// DebitAccount atomically deducts an amount from the specified account.
//
// Invariants:
// The account balance will never transition below zero.
// If the operation fails, no mutation is committed to the persistence tier.
//
// Errors:
// DebitAccount returns ErrNegativeAmount if amount <= 0.
// DebitAccount returns ErrAccountNotFound if the target account does not exist.
// DebitAccount returns ErrInsufficientFunds if account balance < amount.
// DebitAccount returns context.Canceled or context.DeadlineExceeded if ctx expires before commit.
func DebitAccount(ctx context.Context, accountID string, amount int64) (*AccountState, error) {
	// @impl:FR-LEDGER-02
	if amount <= 0 {
		return nil, ErrNegativeAmount
	}
	// Execution logic omitted for template
	return &AccountState{
		ID:        accountID,
		Balance:   1000,
		UpdatedAt: time.Now().UTC(),
	}, nil
}
