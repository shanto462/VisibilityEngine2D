using System.Windows;
using System.Windows.Threading;

namespace VisibilityIn2DGrid.Extensions;

public static class WindowExtensions
{
    public static void RunOnUIThread(this Window window, Action action, DispatcherPriority priority = DispatcherPriority.Render)
    {
        window.Dispatcher.Invoke(priority, action);
    }

    public static void RunWithProgressBar(this Window window, Func<Task> action)
    {
        var progressWindow = new ProgressWindow
        {
            Owner = window
        };

        Task.Run(async () =>
        {
            await Task.Delay(1500);
            try
            {
                await action();
            }
            catch (Exception ex)
            {
                MessageBox.Show(ex.Message);
            }
        }).ContinueWith(task =>
        {
            progressWindow.Close();
            window.Focus();
        }, CancellationToken.None, TaskContinuationOptions.None, TaskScheduler.FromCurrentSynchronizationContext());
        progressWindow.ShowDialog();
    }

    public static void DoEvents(this Window window)
    {
        window.Dispatcher.Invoke(DispatcherPriority.Background, new Action(delegate { }));
    }
}
