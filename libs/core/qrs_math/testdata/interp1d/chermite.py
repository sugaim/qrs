import sys, os, json
import scipy.interpolate as spi
import matplotlib.pyplot as plt
import pandas as pd


def gen_test_data(inpath: str):
  # parse
  with open(inpath, 'r') as f:
    data = json.load(f)
  xs = data['xs']
  ys = data['ys']
  dydxs = data['dydxs']

  # instantinate
  spline = spi.CubicHermiteSpline(xs, ys, dydxs)
  der1 = spline.derivative(nu=1)
  der2 = spline.derivative(nu=2)
  argmin = min(xs) - 2.0
  argmax = max(xs) + 2.0
  args = []
  cursor = argmin
  while cursor < argmax:
    args.append(cursor)
    cursor += 0.05
  
  evaluated = spline(args)
  evaluated_der1 = der1(args)
  evaluated_der2 = der2(args)

  res = []
  for arg, val, der1, der2 in zip(args, evaluated, evaluated_der1, evaluated_der2):
    res.append([arg, val, der1, der2])
  
  coeffs = spline.c
  _, n_polys = coeffs.shape
  coeffs = []
  for i in range(n_polys):
    coeffs.append({
      f"{3 - k}th": c
      for k, c in enumerate(spline.c[:, i])
    })

  return {
    "coefficients": coeffs,
    "evalated": res,
  }


if __name__ == '__main__':
  cases = [
    "chermite.CatmullRom.fwd",
    "chermite.CatmullRom.bwd",
    "chermite.CatmullRom.cen",
  ]
  
  rootdir = os.path.dirname(os.path.abspath(__file__))
  for case in cases:
    inpath = os.path.join(rootdir, case + ".in.json")
    outpath = os.path.join(rootdir, case + ".out.json")
    res = gen_test_data(inpath)
    with open(outpath, 'w') as f:
      json.dump(res, f, indent=2)
    print(f"Generated {outpath}")