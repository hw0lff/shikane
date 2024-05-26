use core::marker::PhantomData;

/// A trait representing a single stage in a [`Pipeline`].
pub trait PipeStage {
    type Input;
    type Output;
    type Error;
    fn process(input: Self::Input) -> Result<Self::Output, Self::Error>;
}

/// An abstract processing pipeline, composed of one or multiple [`PipeStage`]s.
pub struct Pipeline<I, O, E, P>
where
    P: PipeStage<Input = I, Output = O, Error = E>,
{
    _pd: PhantomData<(I, O, E, P)>,
}

/// An internal intermediate stage, acting as glue between two [`PipeStage`]s.
struct Intermediate<A, B, C, E, PAB, PBC>
where
    PAB: PipeStage<Input = A, Output = B, Error = E>,
    PBC: PipeStage<Input = B, Output = C, Error = E>,
{
    _pd: PhantomData<(A, B, C, E, PAB, PBC)>,
}

impl<A, B, E, PAB> Pipeline<A, B, E, PAB>
where
    PAB: PipeStage<Input = A, Output = B, Error = E>,
{
    /// Constructs a new pipeline with the provided initial stage, and returns it.
    pub fn new(_pipe: PAB) -> Self {
        Self { _pd: PhantomData }
    }

    /// Executes the pipeline with the given input data, returning the output.
    pub fn execute(self, input: A) -> Result<B, E> {
        PAB::process(input)
    }

    /// Adds a new stage to the pipeline, returning a new pipeline with the additional stage.
    pub fn add_pipe<C, PBC>(
        self,
        _pipe: PBC,
    ) -> Pipeline<A, C, E, impl PipeStage<Input = A, Output = C, Error = E>>
    where
        PBC: PipeStage<Input = B, Output = C, Error = E>,
    {
        Pipeline::<A, C, _, Intermediate<A, B, C, E, PAB, PBC>> { _pd: PhantomData }
    }
}

impl<A, B, C, E, PAB, PBC> PipeStage for Intermediate<A, B, C, E, PAB, PBC>
where
    PAB: PipeStage<Input = A, Output = B, Error = E>,
    PBC: PipeStage<Input = B, Output = C, Error = E>,
{
    type Input = A;
    type Output = C;
    type Error = E;

    fn process(input: Self::Input) -> Result<Self::Output, Self::Error> {
        PBC::process(PAB::process(input)?)
    }
}

#[cfg(test)]
mod test {
    use core::any::type_name;

    use super::*;

    #[allow(unused)]
    #[derive(Debug, PartialEq)]
    enum PipelineError {
        ProblemWithStep1,
        ProblemWithStep2(InnerErrorStep2),
        ProblemWithStep3,
    }

    #[derive(Debug, PartialEq)]
    struct InnerErrorStep2(usize);

    struct FirstStep;
    struct SecondStep;
    struct ThirdStep;
    struct FailingStep;
    struct RepeatingStep;

    #[derive(Debug, PartialEq, Eq)]
    struct A;
    #[derive(Debug, PartialEq, Eq)]
    struct B;
    #[derive(Debug, PartialEq, Eq)]
    struct C;
    #[derive(Debug, PartialEq, Eq)]
    struct D;

    impl PipeStage for FirstStep {
        type Input = A;
        type Output = B;
        type Error = PipelineError;

        fn process(input: Self::Input) -> Result<Self::Output, PipelineError> {
            print!("got {:?}, ", input);
            println!("returning {}", type_name::<Self::Output>());
            Ok(B)
        }
    }

    impl PipeStage for SecondStep {
        type Input = B;
        type Output = C;
        type Error = PipelineError;

        fn process(input: Self::Input) -> Result<Self::Output, PipelineError> {
            print!("got {:?}, ", input);
            println!("returning {}", type_name::<Self::Output>());
            Ok(C)
        }
    }

    impl PipeStage for ThirdStep {
        type Input = C;
        type Output = D;
        type Error = PipelineError;

        fn process(input: Self::Input) -> Result<Self::Output, PipelineError> {
            print!("got {:?}, ", input);
            println!("returning {}", type_name::<Self::Output>());
            Ok(D)
        }
    }

    impl PipeStage for FailingStep {
        type Input = B;
        type Output = C;
        type Error = PipelineError;

        fn process(input: Self::Input) -> Result<Self::Output, PipelineError> {
            print!("got {:?}, ", input);
            println!("returning {}", type_name::<Self::Output>());
            Err(PipelineError::ProblemWithStep2(InnerErrorStep2(1234)))
        }
    }

    impl PipeStage for RepeatingStep {
        type Input = A;
        type Output = A;
        type Error = PipelineError;

        fn process(input: Self::Input) -> Result<Self::Output, PipelineError> {
            print!("got {:?}, ", input);
            println!("returning {}", type_name::<Self::Output>());
            Ok(A)
        }
    }

    #[test]
    fn test_pipe() {
        let p: Pipeline<_, _, _, _> = Pipeline::new(FirstStep)
            .add_pipe(SecondStep)
            .add_pipe(ThirdStep);
        let res = p.execute(A);
        println!("got {:?}", res);
        assert_eq!(res, Ok(D))
    }

    #[test]
    fn test_pipe_error() {
        let p: Pipeline<_, _, _, _> = Pipeline::new(FirstStep)
            .add_pipe(FailingStep)
            .add_pipe(ThirdStep);
        let res = p.execute(A);
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err(),
            PipelineError::ProblemWithStep2(InnerErrorStep2(1234))
        );
    }

    #[test]
    fn test_repeating_pipe() {
        let p: Pipeline<_, _, _, _> = Pipeline::new(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            .add_pipe(RepeatingStep)
            // .add_pipe(RepeatingStep) // Uncomment to hit recursion limit in rustc
            .add_pipe(RepeatingStep);
        let res = p.execute(A);
        assert_eq!(res, Ok(A));
    }
}
