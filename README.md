Native field arithmetics for SW curves
---

This repo implements native field arithmetics for short Weierstrass curves, using a nice trick from [Tianyi Liu](https://liutianyi.site/).

# Gate config
The gate configuration is:

|   op codes  | cost | q_ec_disabled | q1 | q2 | statement
| ----------- |:----:|:-------------:| -- | -- | -------------
|      ec add |   3  |       0       | 1  | 0  | (x1, y1), (x2, y2) and (x3, -y3) are on a same line
|   ec double |   2  |       0       | 1  | 1  | (x1, y1) and (x3, -y3) are on a tangential line of the curve
| is on curve |   2  |       0       | 0  | 1  | y1^2 = x1^3 - C::b()
|     partial |   3  |       1       | 0  | 1  | y3 = x1 + y1 + x2 + y2 + x3 and
|   decompose |      |               |    |    | x1, y1, x2, y2 are all binary
|         add |   2  |       1       | 1  | 0  | a1 = a0 + b0
|         mul |   2  |       1       | 1  | 1  | a1 = a0 * b0  
# EC ops
## Addition

|index  |  a   |  b   | q_ec | q1 | q2 
|-------|------|------|------|----|----
|       | p1.x | p1.y |   0  | 1  | 0  
|       | p2.x | p2.y |      |    |    
|offset | p3.x | p3.y |      |    |    

An addition is correct if 
- p3 is on curve
- p3 satisfies (x2-x1)/(y2-y1) = (x3-x1)/(-y3-y1)

## Doubling
|index  |  a   |  b   | q_ec | q1 | q2 
|-------|------|------|------|----|----
|       | p1.x | p1.y |   0  | 1  | 1  
|offset | p3.x | p3.y |      |    |    
A doubling is correct if 
- p3 is on curve
- p3 satisfies 2y1 * (y3 + y1) + 3x1^2 * (x3 - x1) = 0

## On Curve
|index  |  a   |  b   | q_ec | q1 | q2 
|-------|------|------|------|----|----
|       | p1.x | p1.y |   0  | 0  | 1  
|offset | res  |      |      |    |    
# Field ops

## partial_bit_decomp

|index  |  a   |  b   | q_ec | q1 | q2 
|-------|------|------|------|----|----
|       |  x1  |  y1  |   1  | 1  | 0  
|       |  x2  |  y2  |      |    |    
|offset |  x3  |  y3  |      |    |    

Assertions:
- x3 = x1 + y1 + x2 + y2 + y3
- x1, y1, x2, y2 are all binary