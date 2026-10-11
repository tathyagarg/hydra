; Raw assembly instructions for ARM64 architecture, compiled manually into
; summer.o for testing purposes.

  MOVZ X0, #0
  MOVZ X1, #1
loop:
  ADD X0, X0, X1
  ADD X1, X1, #1
  CMP X1, #101
  B.LT loop
