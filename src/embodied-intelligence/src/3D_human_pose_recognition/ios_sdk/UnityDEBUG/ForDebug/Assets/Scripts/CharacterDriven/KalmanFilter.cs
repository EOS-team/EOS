using System;
using System.Collections;
using System.Collections.Generic;
using System.Runtime.CompilerServices;
using UnityEngine;
public class KalmanFilter
{
    // Fields
    private int effectiveCount;
    private DiscreteKalmanFilter<ConstantVelocity3DModel, Vector3> kalmanFilter;
    private const int StartCount = 50;

    // Methods
    public KalmanFilter(float timeInterval, float noise)
    {
        ConstantVelocity3DModel initialState = new ConstantVelocity3DModel
        {
            Position = new Vector3(0f, 0f, 0f),
            Velocity = new Vector3(0f, 0f, 0f)
        };
        float[,] processNoise = ConstantVelocity3DModel.GetProcessNoise(noise, timeInterval);
        int measurementVectorDimension = 3;
        int controlVectorDimension = 0;
        Func<ConstantVelocity3DModel, float[]> stateConvertFunc = new Func<ConstantVelocity3DModel, float[]>(ConstantVelocity3DModel.ToArray);
        Func<float[], ConstantVelocity3DModel> stateConvertBackFunc = new Func<float[], ConstantVelocity3DModel>(ConstantVelocity3DModel.FromArray);
        Func<Vector3, float[]> measurementConvertFunc = Testc.test930??(Testc.test930 = new Func<Vector3, float[]>(Testc.test9.b30));
        this.kalmanFilter = new DiscreteKalmanFilter<ConstantVelocity3DModel, Vector3>(initialState, processNoise, measurementVectorDimension, controlVectorDimension, stateConvertFunc, stateConvertBackFunc, measurementConvertFunc);
        this.kalmanFilter.ProcessNoise = ConstantVelocity3DModel.GetProcessNoise(noise, timeInterval);
        this.kalmanFilter.MeasurementNoise = MatrixF.Diagonal<float>(this.kalmanFilter.MeasurementVectorDimension, 1f);
        this.kalmanFilter.MeasurementMatrix = ConstantVelocity3DModel.GetPositionMeasurementMatrix();
        this.kalmanFilter.TransitionMatrix = ConstantVelocity3DModel.GetTransitionMatrix(timeInterval);
        this.kalmanFilter.Predict();
    }

    public void Correct(Vector3 kp)
    {
        this.kalmanFilter.Correct(kp);
    }

    public Vector3 CorrectAndPredict(Vector3 kp)
    {
        this.kalmanFilter.Correct(kp);
        this.kalmanFilter.Predict();
        if (this.effectiveCount < 50)
        {
            this.effectiveCount++;
            return kp;
        }
        return this.GetPosition();
    }

    public Vector3 GetPosition()
    {
        return this.kalmanFilter.State.Position;
    }

    public void Predict()
    {
        this.kalmanFilter.Predict();
    }

    public void UpdateFilterParameter(float timeInterval, float noise)
    {
        this.kalmanFilter.ProcessNoise = ConstantVelocity3DModel.GetProcessNoise(noise, timeInterval);
        this.kalmanFilter.TransitionMatrix = ConstantVelocity3DModel.GetTransitionMatrix(timeInterval);
        this.effectiveCount = 0;
    }

    // Nested Types
    [Serializable, CompilerGenerated]
    private sealed class Testc
    {
        // Fields
        public static readonly KalmanFilter.Testc test9 = new KalmanFilter.Testc();
        public static Func<Vector3, float[]> test930;

        // Methods
        internal float[] b30(Vector3 v)
        {
            return new float[] { v.x, v.y, v.z };
        }
    }

 

}
