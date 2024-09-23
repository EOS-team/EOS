using System;
public class DiscreteKalmanFilter<TState, TMeasurement> : BaseKalmanFilter<TState, TMeasurement>
{
    // Methods
    public DiscreteKalmanFilter(TState initialState, float[,] initialStateError, int measurementVectorDimension, int controlVectorDimension, Func<TState, float[]> stateConvertFunc, Func<float[], TState> stateConvertBackFunc, Func<TMeasurement, float[]> measurementConvertFunc) : base(initialState, initialStateError, measurementVectorDimension, controlVectorDimension, stateConvertFunc, stateConvertBackFunc, measurementConvertFunc)
    {
    }

    private void correct(float[] innovationVector)
    {
        if (innovationVector.Length != base.MeasurementVectorDimension)
        {
            throw new Exception("PredicitionError error vector (innovation vector) must have the same length as measurement.");
        }
        base.state = base.state.Add(base.KalmanGain.Multiply(innovationVector));
        float[,] a = MatrixF.Identity(base.StateVectorDimension);
        base.EstimateCovariance = a.Subtract(base.KalmanGain.Multiply(base.MeasurementMatrix), false).Multiply(base.EstimateCovariance.Transpose<float>());
    }

    protected override void CorrectInternal(float[] measurement)
    {
        float[] innovationVector = base.CalculateDelta(measurement);
        this.correct(innovationVector);
    }

    protected override void predictInternal(float[] controlVector)
    {
        base.state = base.TransitionMatrix.Multiply(base.state);
        if (controlVector != null)
        {
            base.state = base.state.Add(base.ControlMatrix.Multiply(controlVector));
        }
        base.EstimateCovariance = base.TransitionMatrix.Multiply(base.EstimateCovariance).Multiply(base.TransitionMatrix.Transpose<float>()).Add(base.ProcessNoise);
        float[,] b = base.MeasurementMatrix.Transpose<float>();
        base.ResidualCovariance = base.MeasurementMatrix.Multiply(base.EstimateCovariance).Multiply(b).Add(base.MeasurementNoise);
        base.ResidualCovarianceInv = base.ResidualCovariance.Inverse();
        base.KalmanGain = base.EstimateCovariance.Multiply(b).Multiply(base.ResidualCovarianceInv);
    }
}


