using System;
using System.Collections;
using System.Collections.Generic;
using UnityEngine;

public sealed class OneEuroFilter
{
    // Fields
    private float beta;
    private Vector3 d_cutoff;
    private Vector3 dx_prev;
    private Vector3 min_cutoff;
    private float t_prev;
    private Vector3 x_prev;

    // Methods
    public OneEuroFilter()
    {
        this.min_cutoff = new Vector3(3f, 3f, 3f);
        this.beta = 0.1f;
        this.d_cutoff = new Vector3(1f, 1f, 1f);
    }

    public OneEuroFilter(float min_cutoff, float beta, float d_cutoff)
    {
        this.min_cutoff = new Vector3(3f, 3f, 3f);
        this.beta = 0.1f;
        this.d_cutoff = new Vector3(1f, 1f, 1f);
        this.min_cutoff = new Vector3(min_cutoff, min_cutoff, min_cutoff);
        this.beta = beta;
        this.d_cutoff = new Vector3(d_cutoff, d_cutoff, d_cutoff);
    }

    private Vector3 AbsVector3(Vector3 v)
    {
        return new Vector3(Mathf.Abs(v.x), Mathf.Abs(v.y), Mathf.Abs(v.z));
    }

    public Vector3 CorrectAndPredict(Vector3 kp, float t)
    {
        float num = t;//- this.t_prev;
        Vector3 a = this.smoothing_factor(num, this.d_cutoff);
        Vector3 x = (Vector3)((kp - this.x_prev) / num);
        Vector3 v = this.exponential_smoothing(a, x, this.dx_prev);
        Vector3 cutoff = this.min_cutoff + ((Vector3)(this.beta * this.AbsVector3(v)));
        Vector3 vector5 = this.smoothing_factor(num, cutoff);
        this.x_prev = kp;
        this.dx_prev = v;
        this.t_prev = t;
        return this.exponential_smoothing(vector5, kp, this.x_prev);
    }

    private Vector3 exponential_smoothing(Vector3 a, Vector3 x, Vector3 x_prev)
    {
        return new Vector3((a.x * x.x) + ((1f - a.x) * x_prev.x), (a.y * x.y) + ((1f - a.y) * x_prev.y), (a.z * x.z) + ((1f - a.z) * x_prev.z));
    }

    private Vector3 smoothing_factor(float t_e, Vector3 cutoff)
    {
        Vector3 vector = (Vector3)((6.283185f * cutoff) * t_e);
        return new Vector3(vector.x / (vector.x + 1f), vector.y / (vector.y + 1f), vector.z / (vector.z + 1f));
    }
}


