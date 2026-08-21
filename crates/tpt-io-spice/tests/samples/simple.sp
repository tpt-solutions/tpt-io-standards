* Simple RC low-pass with an inverter subcircuit
.temp 27
R1 in out 1k TC=0.001
C1 out 0 100n
V1 in 0 DC 5 AC 1
M1 d g s b nmos W=2u L=0.5u
.model nmos nmos (VTO=0.7 KP=120u)
X1 out vdd inv
.subckt inv (w) a y vdd
R1 a y {w}
C1 y 0 10p
.ends
.tran 1u 10m
.ac dec 10 1 1MEG
.end
