using System;
using System.Collections;
using System.Collections.Generic;
using UnityEngine;
public sealed class LowPassFilter
{
    // Fields
    private int effectiveCount;
    private const int FactorialCount = 10;
    private int NOrderLPF;
    private Vector3[] prevPos3D;
    private float Smooth;

    // Methods
    public LowPassFilter()
    {
        this.prevPos3D = new Vector3[10];
        this.NOrderLPF = 7;
        this.Smooth = 0.9f;
    }

    public LowPassFilter(int order, float smooth)
    {
        this.prevPos3D = new Vector3[10];
        this.NOrderLPF = 7;
        this.Smooth = 0.9f;
        this.NOrderLPF = Mathf.Min(order, 10);
        this.Smooth = smooth;
    }

    public Vector3 CorrectAndPredict(Vector3 kp)
    {
        this.prevPos3D[0] = kp;
        for (int i = 1; i < this.NOrderLPF; i++)
        {
            this.prevPos3D[i] = (Vector3)((this.prevPos3D[i] * (1f - this.Smooth)) + (this.prevPos3D[i - 1] * this.Smooth));
        }
        this.prevPos3D[0] = (Vector3)((this.prevPos3D[0] * (1f - this.Smooth)) + (this.prevPos3D[this.NOrderLPF - 1] * this.Smooth));
        if (this.effectiveCount < 10)
        {
            this.effectiveCount++;
            return kp;
        }
        return this.prevPos3D[0];
    }
}


