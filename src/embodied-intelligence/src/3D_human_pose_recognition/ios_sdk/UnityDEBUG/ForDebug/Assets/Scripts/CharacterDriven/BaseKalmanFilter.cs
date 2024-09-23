using System;
using System.Collections;
using System.Collections.Generic;
using UnityEngine;

public abstract class BaseKalmanFilter<TState, TMeasurement>
{
    // Fields
    private Func<TMeasurement, float[]> measurementConvertFunc;
    protected float[] state;
    private Func<float[], TState> stateConvertBackFunc;
    private Func<TState, float[]> stateConvertFunc;

    // Methods
    protected BaseKalmanFilter(TState initialState, float[,] initialStateError, int measurementVectorDimension, int controlVectorDimension, Func<TState, float[]> stateConvertFunc, Func<float[], TState> stateConvertBackFunc, Func<TMeasurement, float[]> measurementConvertFunc)
    {
        this.initalize(initialState, initialStateError, measurementVectorDimension, controlVectorDimension, stateConvertFunc, stateConvertBackFunc, measurementConvertFunc);
    }

    internal float[] CalculateDelta(float[] measurement)
    {
        float[] b = this.MeasurementMatrix.Multiply(this.state);
        return measurement.Subtract(b, false);
    }

    public void Correct(TMeasurement measurement)
    {
        this.CorrectInternal(this.measurementConvertFunc(measurement));
    }

    protected abstract void CorrectInternal(float[] measurement);
    private void initalize(TState initialState, float[,] initialStateError, int measurementVectorDimension, int controlVectorDimension, Func<TState, float[]> stateConvertFunc, Func<float[], TState> stateConvertBackFunc, Func<TMeasurement, float[]> measurementConvertFunc)
    {
        float[] numArray = stateConvertFunc(initialState);
        this.StateVectorDimension = numArray.Length;
        this.MeasurementVectorDimension = measurementVectorDimension;
        this.ControlVectorDimension = controlVectorDimension;
        this.state = numArray;
        this.EstimateCovariance = initialStateError;
        this.stateConvertFunc = stateConvertFunc;
        this.stateConvertBackFunc = stateConvertBackFunc;
        this.measurementConvertFunc = measurementConvertFunc;
    }

    public void Predict()
    {
        this.Predict(null);
    }

    public void Predict(float[] controlVector)
    {
        this.predictInternal(controlVector);
    }

    protected abstract void predictInternal(float[] controlVector);

    // Properties
    public float[,] ControlMatrix { get; set; }

    public int ControlVectorDimension { get; private set; }

    public float[,] EstimateCovariance { get; protected set; }

    public float[,] KalmanGain { get; protected set; }

    public float[,] MeasurementMatrix { get; set; }

    public float[,] MeasurementNoise { get; set; }

    public int MeasurementVectorDimension { get; private set; }

    public float[,] ProcessNoise { get; set; }

    public float[,] ResidualCovariance { get; protected set; }

    public float[,] ResidualCovarianceInv { get; protected set; }

    public TState State
    {
        get
        {
            return this.stateConvertBackFunc(this.state);
        }
    }

    public int StateVectorDimension { get; private set; }

    public float[,] TransitionMatrix { get; set; }
}

