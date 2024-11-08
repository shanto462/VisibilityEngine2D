using System.Diagnostics;

namespace VisibilityIn2DGrid.Helper;

public static class TimingHelper
{
    public static void Time(Action action, string methodName)
    {
        var stopwatch = new Stopwatch();
        stopwatch.Start();

        try
        {
            action();
        }
        finally
        {
            stopwatch.Stop();
            Console.WriteLine($"Method {methodName} took {stopwatch.ElapsedMilliseconds}ms to execute");
        }
    }

    public static T Time<T>(Func<T> func, string methodName)
    {
        var stopwatch = new Stopwatch();
        stopwatch.Start();

        try
        {
            return func();
        }
        finally
        {
            stopwatch.Stop();
            Console.WriteLine($"Method {methodName} took {stopwatch.ElapsedMilliseconds}ms to execute");
        }
    }

    public static async Task TimeAsync(Func<Task> func, string methodName)
    {
        var stopwatch = new Stopwatch();
        stopwatch.Start();

        try
        {
            await func();
        }
        finally
        {
            stopwatch.Stop();
            Console.WriteLine($"Method {methodName} took {stopwatch.ElapsedMilliseconds}ms to execute");
        }
    }

    public static async Task<T> TimeAsync<T>(Func<Task<T>> func, string methodName)
    {
        var stopwatch = new Stopwatch();
        stopwatch.Start();

        try
        {
            return await func();
        }
        finally
        {
            stopwatch.Stop();
            Console.WriteLine($"Method {methodName} took {stopwatch.ElapsedMilliseconds}ms to execute");
        }
    }
}

