using System.Windows;
using System.Windows.Controls;
using System.Windows.Input;
using System.Windows.Media;
using System.Windows.Shapes;
using VisibilityIn2DGrid.Enums;
using VisibilityIn2DGrid.Extensions;
using VisibilityIn2DGrid.Helper.UI;
using VisibilityIn2DGrid.Helper.UI.Constants;
using VisibilityIn2DGrid.Helper.UI.Controllers;

namespace VisibilityIn2DGrid;

public partial class MainWindow : Window
{
    private readonly ViewState _viewState = new();
    private readonly CanvasManager _canvasManager;
    private readonly ZoomController _zoomController;
    private readonly VisualizationController _visualizationController;

    private readonly Ellipse centerCircle;

    public MainWindow()
    {
        InitializeComponent();

        _canvasManager = new CanvasManager(canvas, _scrollViewer);
        _zoomController = new ZoomController(ZoomTransform, ZoomLevelText, _viewState);
        _visualizationController = new VisualizationController(canvas, _viewState, _canvasManager);

        centerCircle = new Ellipse
        {
            Width = ViewConstants.CenterMarkerSize,
            Height = ViewConstants.CenterMarkerSize,
            Fill = new SolidColorBrush(Color.FromArgb(50, 0, 150, 255)),
            Stroke = new SolidColorBrush(Color.FromArgb(200, 0, 100, 200)),
            StrokeThickness = 2
        };

        InitializeEventHandlers();
    }

    private void InitializeEventHandlers()
    {
        canvas.MouseLeftButtonDown += Canvas_MouseLeftButtonDown;
        canvas.MouseLeftButtonUp += Canvas_MouseLeftButtonUp;
        canvas.MouseMove += Canvas_MouseMove;
        canvas.MouseMove += UpdateMousePosition;

        KeyDown += MainWindow_KeyDown;
        Loaded += MainWindow_Loaded;
        SizeChanged += MainWindow_SizeChanged;
    }

    private void MainWindow_KeyDown(object sender, KeyEventArgs e)
    {
        if (!canvas.IsMouseOverCanvasContent(_scrollViewer))
        {
            return;
        }

        if (_viewState.CurrentViewMode is ViewMode.FrustumCulling or ViewMode.OcclusionCulling)
        {
            bool reload = false;

            if (Keyboard.IsKeyDown(Key.LeftAlt) && Keyboard.IsKeyDown(Key.Up))
            {
                _viewState.FrustumFOVAngle += ViewConstants.KeyboardChangeStep;
                reload = true;
            }
            else if (Keyboard.IsKeyDown(Key.LeftAlt) && Keyboard.IsKeyDown(Key.Down))
            {
                _viewState.FrustumFOVAngle -= ViewConstants.KeyboardChangeStep;
                reload = true;
            }
            else if (Keyboard.IsKeyDown(Key.LeftAlt) && Keyboard.IsKeyDown(Key.Left))
            {
                _viewState.FrustumDirection += ViewConstants.KeyboardChangeStep;
                reload = true;
            }
            else if (Keyboard.IsKeyDown(Key.LeftAlt) && Keyboard.IsKeyDown(Key.Right))
            {
                _viewState.FrustumDirection -= ViewConstants.KeyboardChangeStep;
                reload = true;
            }

            if (_viewState.FrustumFOVAngle < 0.0f)
            {
                _viewState.FrustumFOVAngle = 0.0f;
            }
            else if (_viewState.FrustumFOVAngle > 360.0f)
            {
                _viewState.FrustumFOVAngle = 360.0f;
            }

            if (reload)
            {
                _visualizationController.ExecuteAlgorithm();
            }
        }

        e.Handled = true;
    }

    private void ShowRaysToggle_Checked(object sender, RoutedEventArgs e)
    {
        _viewState.ShowRays = ShowRaysToggle.IsChecked ?? false;
        _visualizationController.ExecuteAlgorithm();
    }

    private void UpdateMousePosition(object sender, MouseEventArgs e)
    {
        Point mousePos = e.GetPosition(canvas);
        MousePositionText.Text = $"Position: {mousePos.X:F0}, {mousePos.Y:F0}";
    }

    private void Canvas_MouseLeftButtonDown(object sender, MouseButtonEventArgs e)
    {
        if (e.ChangedButton == MouseButton.Left)
        {
            DateTime clickTime = DateTime.Now;
            if ((clickTime - _viewState.LastClickTime).TotalMilliseconds <= ViewConstants.DoubleClickTimeMs)
            {
                _viewState.Center = e.GetPosition(canvas);
                TransformCenter();
                _visualizationController.ExecuteAlgorithm();
                _viewState.LastClickTime = DateTime.MinValue;
            }
            else
            {
                _viewState.LastClickTime = clickTime;
                _viewState.IsDragging = true;
                _viewState.LastMousePosition = e.GetPosition(_scrollViewer);
                canvas.Cursor = Cursors.Hand;
                _ = ((UIElement)sender).CaptureMouse();
            }
        }
    }

    private void Canvas_MouseLeftButtonUp(object sender, MouseButtonEventArgs e)
    {
        if (e.ChangedButton == MouseButton.Left)
        {
            _viewState.IsDragging = false;
            _viewState.LastMousePosition = null;
            canvas.Cursor = Cursors.Arrow;
            ((UIElement)sender).ReleaseMouseCapture();
        }
    }

    private void Canvas_MouseMove(object sender, MouseEventArgs e)
    {
        if (_viewState.IsDragging && _viewState.LastMousePosition.HasValue)
        {
            Point currentPosition = e.GetPosition(_scrollViewer);
            double deltaX = currentPosition.X - _viewState.LastMousePosition.Value.X;
            double deltaY = currentPosition.Y - _viewState.LastMousePosition.Value.Y;

            _scrollViewer.ScrollToHorizontalOffset(_scrollViewer.HorizontalOffset - deltaX);
            _scrollViewer.ScrollToVerticalOffset(_scrollViewer.VerticalOffset - deltaY);

            _viewState.LastMousePosition = currentPosition;
        }
    }

    private void ScrollViewer_PreviewMouseWheel(object sender, MouseWheelEventArgs e)
    {
        if (Keyboard.Modifiers == ModifierKeys.Control)
        {
            if (e.Delta > 0)
            {
                _zoomController.ZoomIn();
            }
            else
            {
                _zoomController.ZoomOut();
            }

            e.Handled = true;
        }
    }

    private void ZoomInButton_Click(object sender, RoutedEventArgs e)
    {
        _zoomController.ZoomIn();
    }

    private void ZoomOutButton_Click(object sender, RoutedEventArgs e)
    {
        _zoomController.ZoomOut();
    }

    private void ResetZoomButton_Click(object sender, RoutedEventArgs e)
    {
        _zoomController.ResetZoom();
        _canvasManager.CenterCanvas();
    }

    private void MainWindow_Loaded(object sender, RoutedEventArgs e)
    {
        _canvasManager.CenterCanvas();

        _viewState.Center = new Point(ViewConstants.CanvasWidth / 2, ViewConstants.CanvasHeight / 2);
        CanvasSizeText.Text = $"Canvas Size: {ViewConstants.CanvasWidth} x {ViewConstants.CanvasHeight}";

        DrawCenter();
        RefreshScreen();
    }

    private void MainWindow_SizeChanged(object sender, SizeChangedEventArgs e)
    {
        _canvasManager.CenterCanvas();
    }

    private void TransformCenter()
    {
        Canvas.SetLeft(centerCircle, _viewState.Center.X - (ViewConstants.CenterMarkerSize / 2));
        Canvas.SetTop(centerCircle, _viewState.Center.Y - (ViewConstants.CenterMarkerSize / 2));
    }

    private void DrawCenter()
    {
        _ = canvas.Children.Add(centerCircle);
    }

    private void RefreshScreen()
    {
        this.RunWithProgressBar(async () =>
        {
            await Task.Delay(100);

            this.RunOnUIThread(() =>
            {
                _canvasManager.DrawGrid();
                this.DoEvents();
                _canvasManager.ClearObstacles();
                _canvasManager.AddRandomPolygons();
                this.DoEvents();
                _canvasManager.InitializeSpatialIndex();
                _visualizationController.ExecuteAlgorithm();
                TransformCenter();
                this.DoEvents();
            });

            await Task.Delay(500);
        });
    }

    private void RadioButton_Click(object sender, RoutedEventArgs e)
    {
        if (sender is RadioButton radioButton)
        {
            _viewState.CurrentViewMode = radioButton.Name switch
            {
                "shadowCastRB" => ViewMode.ShadowCast,
                "fcRB" => ViewMode.FrustumCulling,
                "ocRB" => ViewMode.OcclusionCulling,
                _ => _viewState.CurrentViewMode
            };
            _visualizationController.ExecuteAlgorithm();
        }
    }

    private void Button_Click(object sender, RoutedEventArgs e)
    {
        RefreshScreen();
    }
}