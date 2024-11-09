using System.Windows;
using System.Windows.Threading;

namespace VisibilityIn2DGrid.Extensions;

public static class WindowExtensions
{
    public static void RunOnUIThread(this Window window, Action action, DispatcherPriority priority = DispatcherPriority.Render)
    {
        _ = window.Dispatcher.Invoke(priority, action);
    }

    public static void RunWithProgressBar(this Window window, Func<Task> action)
    {
        ProgressWindow progressWindow = new()
        {
            Owner = window
        };

        _ = Task.Run(async () =>
        {
            await Task.Delay(1500);
            try
            {
                await action();
            }
            catch (Exception ex)
            {
                _ = MessageBox.Show(ex.Message);
            }
        }).ContinueWith(task =>
        {
            progressWindow.Close();
            _ = window.Focus();
        }, CancellationToken.None, TaskContinuationOptions.None, TaskScheduler.FromCurrentSynchronizationContext());
        _ = progressWindow.ShowDialog();
    }

    public static void DoEvents(this Window window)
    {
        _ = window.Dispatcher.Invoke(DispatcherPriority.Background, new Action(delegate { }));
    }
}
