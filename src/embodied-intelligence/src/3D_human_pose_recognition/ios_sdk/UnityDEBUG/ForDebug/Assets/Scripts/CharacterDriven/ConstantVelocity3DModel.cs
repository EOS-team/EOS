using System;
using System.Collections;
using System.Collections.Generic;
using UnityEngine;

public class ConstantVelocity3DModel : ICloneable
{
    // Fields
    public const int Dimension = 6;
    public Vector3 Position = Vector3.zero;
    public Vector3 Velocity = Vector3.zero;

    // Methods
    public object Clone()
    {
        return new ConstantVelocity3DModel { Position = this.Position, Velocity = this.Velocity };
    }

    public static ConstantVelocity3DModel FromArray(float[] arr)
    {
        return new ConstantVelocity3DModel { Position = new Vector3(arr[0], arr[2], arr[4]), Velocity = new Vector3(arr[1], arr[3], arr[5]) };
    }

    public static float[,] GetPositionMeasurementMatrix()
    {
        return new float[,] { { 1f, 0f, 0f, 0f, 0f, 0f }, { 0f, 0f, 1f, 0f, 0f, 0f }, { 0f, 0f, 0f, 0f, 1f, 0f } };
    }

    public static float[,] GetProcessNoise(float accelerationNoise, float timeInterval = 1f)
    {
        float[,] singleArray1 = new float[6, 3];
        singleArray1[0, 0] = (timeInterval * timeInterval) / 2f;
        singleArray1[1, 0] = timeInterval;
        singleArray1[2, 1] = (timeInterval * timeInterval) / 2f;
        singleArray1[3, 1] = timeInterval;
        singleArray1[4, 2] = (timeInterval * timeInterval) / 2f;
        singleArray1[5, 2] = timeInterval;
        float[,] matrix = singleArray1;
        float[,] b = MatrixF.Diagonal<float>(matrix.ColumnCount<float>(), accelerationNoise);
        return matrix.Multiply(b).Multiply(matrix.Transpose<float>());
    }

    public static float[,] GetTransitionMatrix(float timeInterval = 1f)
    {
        float num = timeInterval;
        float[,] singleArray1 = new float[,] { { 1f, 0f, 0f, 0f, 0f, 0f }, { 0f, 1f, 0f, 0f, 0f, 0f }, { 0f, 0f, 1f, 0f, 0f, 0f }, { 0f, 0f, 0f, 1f, 0f, 0f }, { 0f, 0f, 0f, 0f, 1f, 0f }, { 0f, 0f, 0f, 0f, 0f, 1f } };
        singleArray1[0, 1] = num;
        singleArray1[2, 3] = num;
        singleArray1[4, 5] = num;
        return singleArray1;
    }

    public static float[] ToArray(ConstantVelocity3DModel modelState)
    {
        return new float[] { modelState.Position.x, modelState.Velocity.x, modelState.Position.y, modelState.Velocity.y, modelState.Position.z, modelState.Velocity.z };
    }
}


