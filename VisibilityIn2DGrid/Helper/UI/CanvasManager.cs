using System.Windows.Controls;
using System.Windows.Shapes;
using VisibilityEngine2D.Helper.UI.Constants;
using VisibilityEngine2D.Index;

namespace VisibilityEngine2D.Helper.UI;

public class CanvasManager
{
    private readonly Canvas _canvas;
    private readonly ScrollViewer _scrollViewer;
    private readonly List<Polygon> _obstacles = [];
    private readonly List<Line> _visualizationRays = [];

    private SpatialIndex? _spatialIndex;

    public CanvasManager(Canvas canvas, ScrollViewer scrollViewer)
    {
        _canvas = canvas;
        _scrollViewer = scrollViewer;

        _canvas.Width = ViewConstants.CanvasWidth;
        _canvas.Height = ViewConstants.CanvasHeight;
    }

    public void DrawGrid()
    {
        GridLineGenerator.DrawMajorGridLines(_canvas, ViewConstants.CanvasWidth, ViewConstants.CanvasHeight,
            ViewConstants.GridSize, ViewConstants.GridLineThickness);
    }

    public void ClearObstacles()
    {
        foreach (Polygon child in _obstacles)
        {
            _canvas.Children.Remove(child);
        }
        _obstacles.Clear();
    }

    public void AddRandomPolygons(int count = 500)
    {
        ClearObstacles();

        for (int i = 0; i < count; i++)
        {
            Polygon polygon = RandomPolygonGenerator.GenerateRandomPolygon(
                ViewConstants.CanvasWidth, ViewConstants.CanvasHeight);

            _obstacles.Add(polygon);
            _ = _canvas.Children.Add(polygon);
        }
    }

    public void InitializeSpatialIndex()
    {
        _spatialIndex?.Dispose();
        _spatialIndex = new SpatialIndex(
            (float)ViewConstants.CanvasWidth,
            (float)ViewConstants.CanvasHeight,
            ViewConstants.GridSize);

        foreach (Polygon obstacle in _obstacles)
        {
            _spatialIndex.Insert(obstacle);
        }
    }

    public void CenterCanvas()
    {
        if (_scrollViewer == null)
        {
            return;
        }

        double horizontalOffset = (ViewConstants.CanvasWidth - _scrollViewer.ViewportWidth) / 2;
        double verticalOffset = (ViewConstants.CanvasHeight - _scrollViewer.ViewportHeight) / 2;

        _scrollViewer.ScrollToHorizontalOffset(horizontalOffset);
        _scrollViewer.ScrollToVerticalOffset(verticalOffset);
    }

    public void ClearVisualizationRays()
    {
        foreach (Line ray in _visualizationRays)
        {
            _canvas.Children.Remove(ray);
        }
        _visualizationRays.Clear();
    }

    public void AddVisualizationRay(Line ray)
    {
        _visualizationRays.Add(ray);
        _ = _canvas.Children.Add(ray);
    }

    public SpatialIndex? GetSpatialIndex()
    {
        return _spatialIndex;
    }

    public List<Polygon> GetObstacles()
    {
        return _obstacles;
    }
}
