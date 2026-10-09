# 1. Problem statement

## Phone-based impersonation fraud

Banks and customers still rely heavily on the phone for sensitive actions such as confirming a transfer, resolving a blocked card, or reporting suspicious activity. The phone channel has no built-in way to prove who is calling, which fraudsters exploit in two directions:

- **Fraudster poses as the bank.** The caller spoofs the bank's number, knows some of the customer's personal details (from leaks or social media), and pressures the customer into moving money, revealing credentials, or reading out one-time codes.
- **Fraudster poses as the customer.** The caller convinces a real bank agent to change details or release funds.

Caller ID, security questions and knowledge of personal details can all be spoofed, guessed or bought. None of them is bound to a live, authenticated relationship between the bank and the customer.

## Why existing controls fall short

| Control | Weakness in this scenario |
|---|---|
| Caller ID / displayed number | Can be spoofed. |
| Security questions, personal details | Often exposed through breaches and social engineering. |
| SMS one-time passwords | Customers can be talked into reading them to the caller; SMS can be intercepted or SIM-swapped. |
| "The bank will never call you" advice | Banks do call customers, so the advice cannot be followed consistently. |

## The gap

The customer has no easy, trustworthy way to check that the person on the line is really from their bank, and the agent has no lightweight way to show it. The bank and the customer already share one strongly authenticated channel: the customer's logged-in mobile banking app. Ringseal uses that channel to authenticate the call.

## Goals

1. Let a customer verify, during a live call, that the caller is a genuine bank agent.
2. Require no secret that the customer must hand to the caller.
3. Be easy for banks to integrate: a small REST API plus mobile SDKs.
4. Support many banks on one deployment (multi-tenant) with strict separation.
5. Leave an audit trail of every verification event.
6. Work for Arabic-speaking users (Arabic UI support in the SDKs and demos).

## Non-goals

- Replacing authentication for the underlying banking transaction.
- Detecting fraud from call content or audio.
- Protecting a customer whose phone or banking app is already compromised.
