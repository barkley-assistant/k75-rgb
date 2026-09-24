# Live test plan — one sitting, 4 batches

Everything here is derived from `analysis/disasm_v2.txt`. No invented bytes.
Each batch ends with a question you answer from watching the keyboard.

Keyboard is currently solid blue. Device: `258a:019d` SINO WEALTH, interface 1.
All commands run from `tools/hidra_probe/` with the keyboard in a known state.

---

## Batch 1 — does anything live respond at all? (~1 min)

    sudo ./target/release/fx sweep

Sends three effect command blocks (op 0x10, 0x12, 0x16, flags r5=1 r3=1),
700 ms apart.

**Report:** did the keyboard change at any point? Which one, if any?
- "nothing happened at all" → the staged block isn't reaching the engine;
  we go back to the staging mechanism, not the packet.
- "changed on op X" → we have the live effect path; X is our new base.
- "flickered then reverted" → the block lands but the apply latch isn't held.

## Batch 2 — is the config block even landing? (~2 min)

    sudo ./target/release/setreg2 0x0a 5a f0 07 10
    sudo ./target/release/setreg2 0x0b 01 00 00 00
    sudo ./target/release/rv 0x01

Diagnostic only. Reads back register state after the staged write so we know
whether report-0x06 `'S'` reaches the staging area `0x11C1+` at all.

**Report:** what does `rv 0x01` print? Any change between before/after?

## Batch 3 — mode control (~2 min)

    sudo ./target/release/fx mode 0
    sudo ./target/release/fx mode 1
    sudo ./target/release/fx mode 2
    sudo ./target/release/fx mode 3

Stages `0x0F22` latch values 0x00/0x01/0x02/0x03 — the exact values the
firmware writes in its own `0x0F54` decode chain at 0x3b2/0x3be/0x3ca.

**Report:** did any of the four produce a different effect? (Breathing /
colour-cycle / wave / reactive are the four.)

## Batch 4 — colour regression check (~1 min)

    sudo ./target/release/save_color ff 00 00

Confirms the existing M1 colour path still works after all of the above, and
gives us a known-good baseline to return the keyboard to.

**Report:** did it turn red?

---

## After the four batches

Bring back:
1. For each batch, changed or not — one word each.
2. If anything changed, which command changed it.
3. Any odd behaviour (flicker, delayed, partial, keys out).

That's enough for me to decide the next move without a second round.
