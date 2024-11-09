using System.Windows;
using System.Windows.Controls;
using System.Windows.Input;

namespace VisibilityEngine2D.Extensions;

public static class MouseExtensions
{
    public static bool IsMouseOverCanvasContent(this Canvas canvas, ScrollViewer scrollViewer)
    {
        Point mousePos = GetTrueMousePosition(canvas, scrollViewer);

        double trueX = mousePos.X;
        double trueY = mousePos.Y;

        double totalWidth = scrollViewer.ScrollableWidth + scrollViewer.ViewportWidth;
        double totalHeight = scrollViewer.ScrollableHeight + scrollViewer.ViewportHeight;

        return trueX >= 0 && trueX <= totalWidth &&
               trueY >= 0 && trueY <= totalHeight;
    }

    private static Point GetTrueMousePosition(Canvas canvas, ScrollViewer scrollViewer)
    {
        Point mousePos = Mouse.GetPosition(canvas);
        return new Point(
            mousePos.X + scrollViewer.HorizontalOffset,
            mousePos.Y + scrollViewer.VerticalOffset
        );
    }
}
