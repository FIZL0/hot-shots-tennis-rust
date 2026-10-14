# B268 — "Joining …" names the lobby's code

`net.rs connect` built the status from `short(&addr.id)` after resolving a short code, so a browser host's
lobby `YD2N` read "Joining g9cl...". It now keeps the code it was given when that's a short code and shows it;
only a code-less dial (a native host's ID or ticket) falls back to the ID's first letters.

All three ways in go through `connect`: Join by Code passes the typed code, the public list and Quick pass
`Listing::dial()` (the listing's short code when it has one).

Test: `short_code_on_a_testnet` (ignored, ~20 s) now waits for the guest's "Joining <code>..." status. Passed.

## Not verified / not 1:1
- Front-end only (ours to design); no original to compare.
- Not tried by hand against a real mobile host; the testnet test covers the short-code path.
- A native host listed without a short code still shows its ID's first letters (as the task allows).
