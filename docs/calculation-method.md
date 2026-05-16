# Calculation Method

## Scope

This seed implements a transparent IEEE-style short-circuit calculation workflow using symmetrical components and per-unit sequence networks. It is designed for offline use and for traceable extension by Codex.

## Sequence networks

For each sequence network, enabled branches are modelled as series impedances and enabled sources are modelled as shunt source equivalents behind impedance. The solver builds a nodal admittance matrix for the connected component that contains each faulted bus. A one-per-unit current injection is applied at the bus to determine the driving-point Thevenin impedance.

The solver computes:

- `Z1` positive-sequence Thevenin impedance
- `Z2` negative-sequence Thevenin impedance
- `Z0` zero-sequence Thevenin impedance when a zero-sequence return path exists

## Fault equations

For prefault voltage `V` in per-unit and fault impedance `Zf` in per-unit:

- 3PH: `I = V / (Z1 + Zf)`
- SLG: `Ia = 3V / (Z1 + Z2 + Z0 + 3Zf)`
- LL: line current magnitude is `sqrt(3) * |V / (Z1 + Z2 + Zf)|`
- DLG: positive sequence current is solved using the standard parallel combination of the negative and zero sequence paths, then phase currents are reconstructed using the `a` operator.

## Asymmetry

The current implementation estimates peak and asymmetrical RMS using the equivalent X/R ratio and the DC offset time constant:

`tau = (X/R) / (2πf)`

This is formula-based and does not embed copyrighted IEEE/ANSI multiplying-factor tables. Production duty checks should add licensed or project-approved factor tables as data.

## Units

- Bus voltage is line-to-line kV.
- System base is MVA.
- Impedances are per-unit on the system base unless otherwise noted.
- Fault impedance input is in ohms and is converted to per-unit at the faulted bus using `Zbase = kV² / MVA`.
- Reported fault current is in kA based on `Ibase = MVA / (sqrt(3) * kV)`.
