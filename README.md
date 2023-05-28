Native field curve arithmetics
---

This repo implements native field arithmetics for short Weierstrass curves, using a nice trick from [Tianyi Liu](https://liutianyi.site/).
It is efficient and is __almost generic__ for both short Weierstrass curves and twisted Edward curves.

# Performance

- A group mul takes __`1221` rows, `2` witness columns and `3` selector columns__. Custom gate has a degree of 5 (coset FFT domain = 4N).
- In comparison, [Jellyfish](https://github.com/EspressoSystems/jellyfish/blob/main/relation/src/gadgets/ecc/msm.rs#L94) uses `1865` rows, `5` witness columns and `13` selector columns. Also use degree 5 gates.

# Gate config
The gate configuration is:

|   op codes  | cost | q_ec | q1 | q2 | q3 | statement
| ----------- |:----:|:----:| -- | -- | -- | -------------
| cond ec add |   4  |   1  | 1  | 0  | 0  | (x1, y1), (x2, y2) and (x3, -y3) are on a same line
|   ec double |   2  |   1  | 0  | 1  | 0  | (x1, y1) and (x3, -y3) are on a tangential line of the curve
| is on curve |   1  |   1  | 0  | 1  | 1  | y1^2 = x1^3 - C::b()
|     partial decompose |   3  |   0  | 1  | 0  | 0  | y3 = x1 + y1 + x2 + y2 + x3 and x1, y1, x2, y2 are all binary
|         add |   2  |   0  | 0  | 1  | 0  | a1 = a0 + b0
|         mul |   2  |   0  | 0  | 0  | 1  | a1 = a0 * b0  
# EC ops
## Conditional Addition

|index  |  a   |  b   | q_ec | q1 | q2 | q3 
|-------|------|------|------|----|----|----
|       | p1.x | p1.y |   1  | 1  | 0  | 0  
|       | p2.x | p2.y |      |    |    |
|       | cond |      |      |    |    |
|offset | p3.x | p3.y |      |    |    |

An addition is correct if 
- p3 is on curve
- p3 satisfies (x2-x1)/(y2-y1) = (x3-x1)/(-y3-y1)

If cond == 1 return p3; else return p1

## Doubling
|index  |  a   |  b   | q_ec | q1 | q2 | q3 
|-------|------|------|------|----|----|----
|       | p1.x | p1.y |   1  | 0  | 1  | 0
|offset | p3.x | p3.y |      |    |    |

A doubling is correct if 
- p3 is on curve
- p3 satisfies 2y1 * (y3 + y1) + 3x1^2 * (x3 - x1) = 0

## On Curve
|index  |  a   |  b   | q_ec | q1 | q2 | q3 
|-------|------|------|------|----|----|----
|offset | p1.x | p1.y |   1  | 0  |  0 | 1  

# Field ops

## partial_bit_decomp

|index  |  a   |  b   | q_ec | q1 | q2 | q3 
|-------|------|------|------|----|----|----
|       |  x1  |  y1  |   0  | 1  | 0  | 0
|       |  x2  |  y2  |      |    |    |
|offset |  x3  |  y3  |      |    |    |

Assertions:
- x3 = x1 + 2y1 + 4x2 + 8y2 + 16y3
- x1, y1, x2, y2 are all binary