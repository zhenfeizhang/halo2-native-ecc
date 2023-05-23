Native field arithmetics for SW curves
---

This repo implements native field arithmetics for short Weierstrass curves, using a nice trick from [Tianyi Liu](https://liutianyi.site/).

The gate configuration is:
```
   used for | q1 | q2 | statement
----------- | -- | -- | -------------
     ec add | 1  | 0  | (x1, y1), (x2, y2) and (x3, -y3) are on a same line
  ec double | 1  | 1  | (x1, y1) and (x3, -y3) are on a tangential line of the curve
is on curve | 0  | 1  | y1^2 = x1^3 - 17
  summation | 0  | 1  | 
```
## Addition
```
index  |  a   |  b   | q1 | q2 
-------|------|------|----|----
       | p1.x | p1.y | 1  | 0  
       | p2.x | p2.y |    |    
offset | p3.x | p3.y |    |    
```
An addition is correct if 
- p3 is on curve
- p3 satisfies (x2-x1)/(y2-y1) = (x3-x1)/(-y3-y1)

## Doubling
```
index  |  a   |  b   | q1 | q2 
-------|------|------|----|----
       | p1.x | p1.y | 1  | 1  
offset | p3.x | p3.y |    |    
```
A doubling is correct if 
- p3 is on curve
- p3 satisfies 2y1 * (y3 + y1) + 3x1^2 * (x3 - x1) = 0

## On Curve
```
index  |  a   |  b   | q1 | q2 
-------|------|------|----|----
       | p1.x | p1.y | 1  | 2  
offset | res  |      |    |    
```

