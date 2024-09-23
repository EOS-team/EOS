using System;
public static class MatrixF
{
    // Methods
    public static float[,] Add(this float[,] a, float[,] b)
    {
        if ((a.GetLength(0) != b.GetLength(0)) || (a.GetLength(1) != b.GetLength(1)))
        {
            throw new ArgumentException("Matrix dimensions must match", "b");
        }
        int length = a.GetLength(0);
        int num2 = a.GetLength(1);
        int num1 = a.Length;
        float[,] numArray = new float[length, num2];
        for (int i = 0; i < num2; i++)
        {
            for (int j = 0; j < length; j++)
            {
                numArray[i, j] = a[i, j] + b[i, j];
            }
        }
        return numArray;
    }

    public static float[] Add(this float[] a, float[] b)
    {
        if (a == null)
        {
            throw new ArgumentNullException("a");
        }
        if (b == null)
        {
            throw new ArgumentNullException("b");
        }
        if (a.Length != b.Length)
        {
            throw new ArgumentException("Vector lengths must match", "b");
        }
        float[] numArray = new float[a.Length];
        for (int i = 0; i < a.Length; i++)
        {
            numArray[i] = a[i] + b[i];
        }
        return numArray;
    }

    public static int ColumnCount<T>(this T[,] matrix)
    {
        return matrix.GetLength(1);
    }

    public static T[,] Diagonal<T>(int size, T value)
    {
        T[,] localArray = new T[size, size];
        for (int i = 0; i < size; i++)
        {
            localArray[i, i] = value;
        }
        return localArray;
    }

    public static float[,] Identity(int size)
    {
        float[,] numArray = new float[size, size];
        for (int i = 0; i < size; i++)
        {
            numArray[i, i] = 1f;
        }
        return numArray;
    }

    public static float[,] Inverse(this float[,] matrix)
    {
        return matrix.Inverse(false);
    }

    public static float[,] Inverse(this float[,] matrix, bool inPlace)
    {
        int length = matrix.GetLength(0);
        int num2 = matrix.GetLength(1);
        if (length != num2)
        {
            throw new ArgumentException("Matrix must be square", "matrix");
        }
        if (length == 3)
        {
            float num3 = matrix[0, 0];
            float num4 = matrix[0, 1];
            float num5 = matrix[0, 2];
            float num6 = matrix[1, 0];
            float num7 = matrix[1, 1];
            float num8 = matrix[1, 2];
            float num9 = matrix[2, 0];
            float num10 = matrix[2, 1];
            float num11 = matrix[2, 2];
            float num12 = ((num3 * ((num7 * num11) - (num8 * num10))) - (num4 * ((num6 * num11) - (num8 * num9)))) + (num5 * ((num6 * num10) - (num7 * num9)));
            if (num12 == 0f)
            {
                throw new Exception();
            }
            float num13 = 1f / num12;
            float[,] singleArray1 = inPlace ? matrix : ((float[,])new float[3, 3]);
            singleArray1[0, 0] = num13 * ((num7 * num11) - (num8 * num10));
            singleArray1[0, 1] = num13 * ((num5 * num10) - (num4 * num11));
            singleArray1[0, 2] = num13 * ((num4 * num8) - (num5 * num7));
            singleArray1[1, 0] = num13 * ((num8 * num9) - (num6 * num11));
            singleArray1[1, 1] = num13 * ((num3 * num11) - (num5 * num9));
            singleArray1[1, 2] = num13 * ((num5 * num6) - (num3 * num8));
            singleArray1[2, 0] = num13 * ((num6 * num10) - (num7 * num9));
            singleArray1[2, 1] = num13 * ((num4 * num9) - (num3 * num10));
            singleArray1[2, 2] = num13 * ((num3 * num7) - (num4 * num6));
            return singleArray1;
        }
        if (length != 2)
        {
            throw new ArgumentException("Matrix not Support size", "matrix");
        }
        float num14 = matrix[0, 0];
        float num15 = matrix[0, 1];
        float num16 = matrix[1, 0];
        float num17 = matrix[1, 1];
        float num18 = (num14 * num17) - (num15 * num16);
        if (num18 == 0f)
        {
            throw new Exception();
        }
        float num19 = 1f / num18;
        float[,] singleArray2 = inPlace ? matrix : ((float[,])new float[2, 2]);
        singleArray2[0, 0] = num19 * num17;
        singleArray2[0, 1] = -num19 * num15;
        singleArray2[1, 0] = -num19 * num16;
        singleArray2[1, 1] = num19 * num14;
        return singleArray2;
    }

    public static float[,] Multiply(this float[,] a, float[,] b)
    {
        float[,] result = new float[a.GetLength(0), b.GetLength(1)];
        a.Multiply(b, result);
        return result;
    }

    public static float[] Multiply(this float[,] matrix, float[] columnVector)
    {
        int length = matrix.GetLength(0);
        if (matrix.GetLength(1) != columnVector.Length)
        {
            throw new Exception("columnVector Vector must have the same length as columns in the matrix.");
        }
        float[] numArray = new float[length];
        for (int i = 0; i < length; i++)
        {
            for (int j = 0; j < columnVector.Length; j++)
            {
                numArray[i] += matrix[i, j] * columnVector[j];
            }
        }
        return numArray;
    }

    public static void Multiply(this float[,] a, float[,] b, float[,] result)
    {
        int length = result.GetLength(0);
        int num2 = result.GetLength(1);
        float[] numArray = new float[a.GetLength(1)];
        for (int i = 0; i < num2; i++)
        {
            for (int j = 0; j < numArray.Length; j++)
            {
                numArray[j] = b[j, i];
            }
            for (int k = 0; k < length; k++)
            {
                float num6 = 0f;
                for (int m = 0; m < numArray.Length; m++)
                {
                    num6 += a[k, m] * numArray[m];
                }
                result[k, i] = num6;
            }
        }
    }

    public static float[,] Subtract(this float[,] a, float[,] b, bool inPlace = false)
    {
        if (a == null)
        {
            throw new ArgumentNullException("a");
        }
        if (b == null)
        {
            throw new ArgumentNullException("b");
        }
        if ((a.GetLength(0) != b.GetLength(0)) || (a.GetLength(1) != b.GetLength(1)))
        {
            throw new ArgumentException("Matrix dimensions must match", "b");
        }
        int length = a.GetLength(0);
        int num2 = b.GetLength(1);
        int num1 = a.Length;
        float[,] numArray = inPlace ? a : ((float[,])new float[length, num2]);
        for (int i = 0; i < length; i++)
        {
            for (int j = 0; j < num2; j++)
            {
                numArray[i, j] = a[i, j] - b[i, j];
            }
        }
        return numArray;
    }

    public static float[] Subtract(this float[] a, float[] b, bool inPlace = false)
    {
        if (a.Length != b.Length)
        {
            throw new ArgumentException("Vector length must match", "b");
        }
        float[] numArray = inPlace ? a : new float[a.Length];
        for (int i = 0; i < a.Length; i++)
        {
            numArray[i] = a[i] - b[i];
        }
        return numArray;
    }

    public static T[,] Transpose<T>(this T[,] matrix)
    {
        return matrix.Transpose<T>(false);
    }

    public static T[,] Transpose<T>(this T[,] matrix, bool inPlace)
    {
        int length = matrix.GetLength(0);
        int num2 = matrix.GetLength(1);
        if (inPlace)
        {
            if (length != num2)
            {
                throw new ArgumentException("Only square matrices can be transposed in place.", "matrix");
            }
            for (int j = 0; j < length; j++)
            {
                for (int k = j; k < num2; k++)
                {
                    T local = matrix[k, j];
                    matrix[k, j] = matrix[j, k];
                    matrix[j, k] = local;
                }
            }
            return matrix;
        }
        T[,] localArray = new T[num2, length];
        for (int i = 0; i < length; i++)
        {
            for (int m = 0; m < num2; m++)
            {
                localArray[m, i] = matrix[i, m];
            }
        }
        return localArray;
    }
}
