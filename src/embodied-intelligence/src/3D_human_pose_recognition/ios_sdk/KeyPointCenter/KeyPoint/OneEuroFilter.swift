//
//  OneEuroFilter.swift
//  MetaScripts
//
//  Created by ZhaoTianyu on 2024/7/10.
//

import Foundation

final class OneEuroFilter {
    // Fields
    private var beta: Float
    private var d_cutoff: Vector3
    private var dx_prev = Vector3(x: 0, y: 0, z: 0)
    private var min_cutoff: Vector3
    private var t_prev: Float = 0
    private var x_prev = Vector3(x: 0, y: 0, z: 0)

    // Methods
    init() {
        self.min_cutoff = Vector3(x: 3, y: 3, z: 3)
        self.beta = 0.1
        self.d_cutoff = Vector3(x: 1, y: 1, z: 1)
    }

    init(min_cutoff: Float, beta: Float, d_cutoff: Float) {
        self.min_cutoff = Vector3(x: 3, y: 3, z: 3)
        self.beta = 0.1
        self.d_cutoff = Vector3(x: 1, y: 1, z: 1)
        self.min_cutoff = Vector3(x: min_cutoff, y: min_cutoff, z: min_cutoff)
        self.beta = beta
        self.d_cutoff = Vector3(x: d_cutoff, y: d_cutoff, z: d_cutoff)
    }

    private func AbsVector3(v: Vector3) -> Vector3 {
        return Vector3(x: abs(v.x), y: abs(v.y), z: abs(v.z))
    }

    func correctAndPredict(kp: Vector3, t: Float) -> Vector3 {
        let num = t
        let a = self.smoothing_factor(t_e: num, cutoff: self.d_cutoff)
        let x = (kp - self.x_prev) / num
        let v = self.exponential_smoothing(a: a, x: x, x_prev: self.dx_prev)
        let temp = self.AbsVector3(v: v)
        let cutoff = self.min_cutoff + (temp * self.beta)
        let vector5 = self.smoothing_factor(t_e: num, cutoff: cutoff)
        self.x_prev = kp
        self.dx_prev = v
        self.t_prev = t
        return self.exponential_smoothing(a: vector5, x: kp, x_prev: self.x_prev)
    }

    private func exponential_smoothing(a: Vector3, x: Vector3, x_prev: Vector3) -> Vector3 {
        return Vector3(x: (a.x * x.x) + ((1 - a.x) * x_prev.x),
                       y: (a.y * x.y) + ((1 - a.y) * x_prev.y),
                       z: (a.z * x.z) + ((1 - a.z) * x_prev.z))
    }

    private func smoothing_factor(t_e: Float, cutoff: Vector3) -> Vector3 {
        let vector = (cutoff * 6.283185) * t_e
        return Vector3(x: vector.x / (vector.x + 1),
                       y: vector.y / (vector.y + 1),
                       z: vector.z / (vector.z + 1))
    }
}
